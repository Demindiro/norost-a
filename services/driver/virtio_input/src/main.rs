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
static mut BUFFER: Option<dux::Page> = None;
static mut BUFFER_WRITTEN: u16 = 0;

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
		pci::HeaderCommon::COMMAND_INTERRUPT_DISABLE
			| pci::HeaderCommon::COMMAND_MMIO_MASK
			| pci::HeaderCommon::COMMAND_BUS_MASTER_MASK,
	);

	// Set up device
	let alloc = |count| {
		let addr = virt;
		let ret = unsafe { kernel::mem_alloc(addr, count, 0b11) };
		match ret.status {
			0 => {
				virt = virt.wrapping_add(size);
				Ok(dux::Page::new(ptr::NonNull::new(addr).unwrap()).unwrap())
			}
			err => Err(err),
		}
	};
	let dev = virtio::pci::new_device(pci, &virt_bars[..], virtio_input::Device::new, alloc)
		.expect("failed to create device");

	// Add self to registry
	let mut name = [0; 128];
	let name_len = usize::from(dev.name(&mut name));

	let ret = unsafe { kernel::sys_registry_add(name.as_ptr(), name_len.into(), usize::MAX) };
	assert_eq!(ret.status, 0, "failed to add self to registry");

	unsafe { SET = Some(scancode::default()) };

	unsafe {
		DEVICE = Some(dev);
	}

	let ipc = dux::ipc::queue::IPC::<4>::new();

	loop {
		let rx = ipc.receive();

		match dux::ipc::Op::from_flags(rx.flags_user) {
			Ok(dux::ipc::Op::Map) => {
				let mut length = 0;

				unsafe {
					// Wait until data is available
					// TODO this blocks writes from other tasks.
					while BUFFER_WRITTEN == 0 {
						process_events();
						kernel::io_wait(50_000);
					}
				}

				// Send data
				*ipc.transmit() = dux::ipc::Packet {
					name: None,
					name_length: 0,
					flags_user: 0,
					flags_kernel: 0b011_00_10_00, // WR & move data page
					id: 0,
					address: rx.address,
					data: unsafe { BUFFER_WRITTEN },
					data_length: length,
					data_offset: 0,
				};

				// Acquire new page for buffer
			}
			// Just ignore other requests for now
			_ => (),
		}

		// There is no need to free ranges as no free ranges were added anyways.
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
					kernel::sys_log!("unknown event: 0x{:x}", evt.code());
				},
			}
		}
	});
}
