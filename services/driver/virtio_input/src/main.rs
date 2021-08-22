//! # Virtio input driver
//!
//! ## References
//!
//! https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-3390008

#![no_std]
#![no_main]
#![feature(asm)]
#![feature(global_asm)]
#![feature(naked_functions)]
#![feature(panic_info_message)]

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
	kernel::sys_log!("Panic!");
	if let Some(m) = info.message() {
		kernel::sys_log!("  Message: {}", m);
	}
	if let Some(l) = info.location() {
		kernel::sys_log!("  Location: {}", l);
	}
	loop {}
}

mod rtbegin;
mod scancode;

use core::convert::{TryFrom, TryInto};
use core::num::NonZeroU8;
use core::ptr;
use kernel::Page;

/// Write buffer for data read.
///
/// 4 KiB should be quite enough.
static mut BUFFER: [u8; 1 << 12] = [0; 1 << 12];

// We spin it right round baby right round
/// The last index of data read from UART.
static mut NEW_INDEX: u16 = 0;

/// The last index of data read from the buffer
static mut USED_INDEX: u16 = 0;

static mut DEVICE: Option<virtio_input::Device> = None;

static mut SET: Option<scancode::ScanCodes> = None;

static mut KEY_MODIFIERS: KeyModifiers = KeyModifiers(0);

struct KeyModifiers(u8);

unsafe impl Sync for KeyModifiers {}

impl KeyModifiers {
	const LSHIFT: u8 = 0x1;
	const RSHIFT: u8 = 0x2;
	const CAPSLOCK: u8 = 0x4;

	fn set_lshift(&mut self, enable: bool) {
		self.0 &= !Self::LSHIFT;
		self.0 |= Self::LSHIFT * u8::from(enable);
	}

	fn set_rshift(&mut self, enable: bool) {
		self.0 &= !Self::RSHIFT;
		self.0 |= Self::RSHIFT * u8::from(enable);
	}

	fn set_capslock(&mut self, enable: bool) {
		self.0 &= !Self::CAPSLOCK;
		self.0 |= Self::CAPSLOCK * u8::from(enable);
	}

	fn lshift(&self) -> bool {
		self.0 & Self::LSHIFT > 0
	}

	fn rshift(&self) -> bool {
		self.0 & Self::RSHIFT > 0
	}

	fn capslock(&self) -> bool {
		self.0 & Self::CAPSLOCK > 0
	}
}

#[export_name = "main"]
fn main() {
	// FIXME move this to rtbegin
	unsafe { dux::init() };

	// Parse arguments
	let mut pci = None;
	let mut bars = [None; 6];

	driver::parse_args(rtbegin::args(), |arg, _| {
		match arg {
			driver::Arg::Pci(p) => pci
				.replace(p)
				.ok_or(())
				.expect_err("multiple pci addresses specified"),
			driver::Arg::BarMmio(b) => {
				let e = usize::try_from(b.index)
					.ok()
					.and_then(|i| bars.get_mut(i))
					.expect("index out of range");
				e.replace(b)
					.ok_or(())
					.expect_err("bar specified multiple times");
			}
			// Ignore I/O, as we only use MMIO.
			driver::Arg::BarIo(b) => (),
			arg => panic!("bad argument: {:?}", arg),
		}
	})
	.unwrap();

	let mut virt = 0x1000_0000 as *mut kernel::Page;

	// Map PCI header
	let pci = pci.unwrap();
	let addr = usize::try_from(pci.address >> Page::OFFSET_BITS).expect("address out of range");
	let size = usize::try_from(pci.size).expect("size too large");
	let ret = unsafe { kernel::sys_direct_alloc(virt, addr, size / Page::SIZE, 0b11) };
	assert_eq!(ret.status, 0, "failed to map pci header");
	let pci = unsafe { pci::Header::from_raw(virt) };
	virt = virt.wrapping_add(size / Page::SIZE);

	// Map BARs
	let mut virt_bars = [None; 6];
	for (w, r) in virt_bars.iter_mut().zip(bars.iter()) {
		*w = r.map(|b| {
			let addr =
				usize::try_from(b.address >> Page::OFFSET_BITS).expect("address out of range");
			let size = usize::try_from(b.size).expect("size out of range");
			let ret = unsafe { kernel::sys_direct_alloc(virt, addr, size / Page::SIZE, 0b11) };
			assert_eq!(ret.status, 0, "failed to map BAR region");
			let addr = core::ptr::NonNull::new(virt).unwrap();
			virt = virt.wrapping_add(size / Page::SIZE);
			addr.cast()
		});
	}

	pci.set_command(
		pci::HeaderCommon::COMMAND_MMIO_MASK | pci::HeaderCommon::COMMAND_BUS_MASTER_MASK,
	);

	// Set up device
	let dev = virtio::pci::new_device(pci, &virt_bars[..], virtio_input::Device::new)
		.expect("failed to create device");

	// Add self to registry
	let mut name = [0; 128];
	let name_len = usize::from(dev.name(&mut name));
	kernel::dbg!(core::str::from_utf8(&name[..name_len]));

	let ret = unsafe { kernel::sys_registry_add(name.as_ptr(), name_len.into(), usize::MAX) };
	assert_eq!(ret.status, 0, "failed to add self to registry");

	for i in 0..5 {
		let name_len = usize::from(dev.ev_bits(&mut name, i));
		kernel::dbg!(format_args!("{:x?}", &name[..name_len]));
	}

	unsafe { SET = Some(scancode::default()) };

	unsafe {
		DEVICE = Some(dev);
	}

	loop {
		let rx = dux::ipc::receive();

		match kernel::ipc::Op::try_from(rx.opcode.unwrap()) {
			Ok(kernel::ipc::Op::Read) => {
				// Figure out object to read.
				let data = unsafe {
					core::slice::from_raw_parts_mut(rx.data.unwrap().as_ptr().cast(), rx.length)
				};
				let path = rx.name.map(|name| unsafe {
					core::slice::from_raw_parts(name.cast::<u8>().as_ptr(), rx.name_len.into())
				});

				let mut length = 0;

				unsafe {
					// Wait until data is available
					// TODO this blocks writes from other tasks.
					while USED_INDEX == NEW_INDEX {
						process_events();
						kernel::io_wait(50_000);
					}

					while USED_INDEX != NEW_INDEX && length < data.len() {
						data[length] = BUFFER[usize::from(USED_INDEX) & (BUFFER.len() - 1)];
						USED_INDEX = USED_INDEX.wrapping_add(1);
						length += 1;
					}
				}

				// Send completion event
				*dux::ipc::transmit() = kernel::ipc::Packet {
					uuid: kernel::ipc::UUID::INVALID,
					opcode: Some(kernel::ipc::Op::Read.into()),
					name: None,
					name_len: 0,
					flags: 0,
					id: 0,
					address: rx.address,
					data: None,
					length,
					offset: 0,
				};
			}
			// Just ignore other requests for now
			_ => (),
		}

		// Free ranges
		if let Some(data) = rx.data {
			let len = dux::Page::min_pages_for_range(rx.length);
			let ret = unsafe { kernel::mem_dealloc(data.as_ptr() as *mut _, len) };
			assert_eq!(ret.status, 0);
			dux::ipc::add_free_range(
				dux::Page::new(core::ptr::NonNull::new(data.as_ptr() as *mut _).unwrap()).unwrap(),
				len,
			)
			.unwrap();
		}
		if let Some(name) = rx.name {
			let len = dux::Page::min_pages_for_range(rx.name_len.into());
			let ret = unsafe { kernel::mem_dealloc(name.as_ptr() as *mut _, len) };
			assert_eq!(ret.status, 0);
			dux::ipc::add_free_range(
				dux::Page::new(core::ptr::NonNull::new(name.as_ptr() as *mut _).unwrap()).unwrap(),
				len,
			)
			.unwrap();
		}
	}
}

fn process_events() {
	let k_mods = unsafe { &mut KEY_MODIFIERS };
	let mut putc = |on: bool, c: char| unsafe {
		if on {
			let len = c.encode_utf8(&mut BUFFER[usize::from(NEW_INDEX)..]).len();
			NEW_INDEX += u16::try_from(len).unwrap();
		}
	};
	unsafe { DEVICE.as_mut().unwrap() }.receive(&mut |evt| {
		if let Some(k) = NonZeroU8::new(evt.code().try_into().unwrap()) {
			use scancode::*;
			let mut mods = Modifiers::new();
			mods.set_caps((k_mods.lshift() || k_mods.rshift()) != k_mods.capslock());
			let on = evt.value() > 0;
			match unsafe { SET.as_mut().unwrap() }.get(mods, k) {
				Some(Key::Char(c)) => putc(on, c),
				Some(Key::LShift) => k_mods.set_lshift(on),
				Some(Key::RShift) => k_mods.set_rshift(on),
				Some(Key::Capslock) => k_mods.set_capslock(on),
				Some(Key::Backspace) => putc(on, '\x08'),
				Some(Key::Enter) => putc(on, '\n'),
				Some(Key::Space) => putc(on, ' '),
				_ => todo!(),
				None => unsafe {
					kernel::dbg!(format_args!("0x{:x}", evt.code()));
				},
			}
		}
	});
}
