//! # Virtio block driver
//!
//! ## References
//!
//! https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-2390002

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

mod notification;
mod rtbegin;

use core::convert::{TryFrom, TryInto};
use core::ptr;
use kernel::Page;

static mut DEVICE: Option<virtio_block::BlockDevice> = None;

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

	let irq = match pci {
		pci::Header::H0(h) => h.interrupt_pin.get(),
		_ => todo!(),
	};

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

	// Route interrupts to us
	{
		//let uuid =  | irq[usize::from(irq)] ;
		let uuid = 0x21;
		let uuid = u128::from(irq);
		*dux::ipc::transmit() = kernel::ipc::Packet {
			address: 1,
			data: None,
			uuid: kernel::ipc::UUID::new(uuid),
			id: 0,
			flags: 0,
			length: 0,
			name: None,
			name_len: 0,
			offset: 0,
			opcode: core::num::NonZeroU8::new(128), // OP_OPEN
		};
	}

	pci.set_command(
		pci::HeaderCommon::COMMAND_MMIO_MASK | pci::HeaderCommon::COMMAND_BUS_MASTER_MASK,
	);

	// TODO move this to behind block device setup but right before we allocate an interrupt.
	notification::init();

	// Set up block device
	let mut device = virtio::pci::new_device(pci, &virt_bars[..], virtio_block::BlockDevice::new)
		.expect("failed to create device");

	// Add self to registry
	let name = "virtio_block";
	let ret = unsafe { kernel::sys_registry_add(name.as_ptr(), name.len(), usize::MAX) };
	assert_eq!(ret.status, 0, "failed to add self to registry");

	// Wait for & respond to requests
	loop {
		let rxq = dux::ipc::receive();
		let op = rxq.opcode.unwrap();

		let ratio = kernel::Page::SIZE / core::mem::size_of::<virtio_block::Sector>();
		let length = rxq.length / virtio_block::Sector::SIZE;
		let offset = rxq.offset * ratio as u64;

		let mut wait = || unsafe { kernel::io_wait(u64::MAX) };

		match kernel::ipc::Op::try_from(op) {
			Ok(kernel::ipc::Op::Read) => {
				let data = unsafe {
					let data = rxq.data.unwrap().as_ptr().cast::<virtio_block::Sector>();
					core::slice::from_raw_parts_mut(data, length)
				};

				device
					.read(data, offset, &mut wait)
					.expect("failed to read sectors");

				// Send completion event
				*dux::ipc::transmit() = kernel::ipc::Packet {
					uuid: kernel::ipc::UUID::INVALID,
					opcode: Some(kernel::ipc::Op::Read.into()),
					name: None,
					name_len: 0,
					flags: 0,
					id: 0,
					address: rxq.address,
					data: None,
					length: length / virtio_block::Sector::SIZE,
					offset: offset / ratio as u64,
				};
			}
			Ok(kernel::ipc::Op::Write) => {
				let data = unsafe {
					let data = rxq.data.unwrap().as_ptr().cast::<virtio_block::Sector>();
					core::slice::from_raw_parts(data, length)
				};

				device
					.write(data, offset, &mut wait)
					.expect("failed to write sectors");

				// Confirm reception.
				*dux::ipc::transmit() = kernel::ipc::Packet {
					uuid: kernel::ipc::UUID::INVALID,
					opcode: Some(kernel::ipc::Op::Write.into()),
					name: None,
					name_len: 0,
					flags: 0,
					id: 0,
					address: rxq.address,
					data: None,
					length: length / virtio_block::Sector::SIZE,
					offset: offset / ratio as u64,
				};
			}
			// Just ignore other requests for now
			_ => (),
		}

		// Free ranges
		if let Some(data) = rxq.data {
			let len = dux::Page::min_pages_for_range(rxq.length);
			let ret = unsafe { kernel::mem_dealloc(data.as_ptr() as *mut _, len) };
			assert_eq!(ret.status, 0);
			dux::ipc::add_free_range(
				dux::Page::new(core::ptr::NonNull::new(data.as_ptr() as *mut _).unwrap()).unwrap(),
				len,
			)
			.unwrap();
		}
		if let Some(name) = rxq.name {
			let len = dux::Page::min_pages_for_range(rxq.name_len.into());
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
