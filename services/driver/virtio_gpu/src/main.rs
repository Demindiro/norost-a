//! # Virtio GPU driver
//!
//! ## References
//!
//! https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-3200007

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

use core::convert::{TryFrom, TryInto};
use core::ptr;
use kernel::Page;

static mut DEVICE: Option<virtio_gpu::Device> = None;

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

	// Set up block device
	let mut device = virtio::pci::new_device(pci, &virt_bars[..], virtio_gpu::Device::new)
		.expect("failed to create device");

	// Create draw buffer
	#[repr(C)]
	struct RGBA8 {
		r: u8,
		g: u8,
		b: u8,
		a: u8,
	}
	let (w, h) = (800, 600);
	let size = (w * h * core::mem::size_of::<RGBA8>() + kernel::Page::MASK) / kernel::Page::SIZE;
	let addr = core::ptr::NonNull::new(0x3333_0000 as *mut _).unwrap();
	let ret = unsafe { kernel::mem_alloc(addr.as_ptr(), size, 0b11) };
	assert_eq!(ret.status, 0);
	let buffer = unsafe {
		let ptr = addr.as_ptr().cast::<RGBA8>();
		let size = size * kernel::Page::SIZE / core::mem::size_of::<RGBA8>();
		// SAFETY: while the device will read from it, only we will write to it.
		core::slice::from_raw_parts_mut(ptr, size)
	};

	// Set up scan
	let rect = virtio_gpu::Rect::new(0, 0, w.try_into().unwrap(), h.try_into().unwrap());
	let ret = unsafe { device.init_scanout(virtio_gpu::Format::RGBA8Unorm, rect, addr, size) };
	ret.unwrap();

	for x in 0..w {
		for y in 0..h {
			let r = (x * 127 / w) as u8;
			let g = (y * 127 / h) as u8;
			buffer[x + y * w] = RGBA8 {
				r: r * 2,
				g: g * 2,
				b: 255 - r - g,
				a: 255,
			};
		}
	}

	core::sync::atomic::fence(core::sync::atomic::Ordering::AcqRel);

	// Draw
	device.draw(rect).expect("failed to draw");

	// Add self to registry
	let name = "virtio_gpu";
	let ret = unsafe { kernel::sys_registry_add(name.as_ptr(), name.len(), usize::MAX) };
	assert_eq!(ret.status, 0, "failed to add self to registry");

	loop {
		let rx = dux::ipc::receive();

		const OP_OPEN: u8 = 128;
		const OP_FLUSH: u8 = 129;

		use core::slice;

		match rx.opcode.map(|n| n.get()).unwrap_or(0) {
			OP_OPEN => {
				kernel::sys_log!("OP_OPEN");
				*dux::ipc::transmit() = kernel::ipc::Packet {
					uuid: kernel::ipc::UUID::INVALID,
					data: Some(addr),
					length: w * h * core::mem::size_of::<RGBA8>(),
					address: rx.address,
					id: rx.id,
					name: None,
					name_len: 0,
					flags: 0,
					offset: 0,
					opcode: rx.opcode,
				};
			}
			OP_FLUSH => {
				kernel::sys_log!("OP_FLUSH");
				device.draw(rect).expect("failed to draw");
			}
			_ => todo!(),
		}

		unsafe {
			kernel::io_wait(u64::MAX);
		}
	}
}
