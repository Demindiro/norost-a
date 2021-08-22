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

mod rtbegin;

use core::convert::{TryFrom, TryInto};
use core::ptr;
use kernel::Page;

static mut DEVICE: Option<virtio_block::BlockDevice> = None;

#[naked]
extern "C" fn notification_handler_entry() {
	unsafe {
		asm!(
			"
			# a0: type
			# a1: value
			# a7: address
			#
			# The original a[0-2] are stored on the stack by the kernel.
			.equ	GP_REGBYTES, 8
			.equ	NOTIFY_RETURN, 9
			addi	sp, sp, -(13 + 4) * GP_REGBYTES
			sd		t0, 0 * GP_REGBYTES (sp)
			sd		t1, 1 * GP_REGBYTES (sp)
			sd		t2, 2 * GP_REGBYTES (sp)
			sd		t3, 3 * GP_REGBYTES (sp)
			sd		t4, 4 * GP_REGBYTES (sp)
			sd		t5, 5 * GP_REGBYTES (sp)
			sd		t6, 6 * GP_REGBYTES (sp)
			sd		a3, 7 * GP_REGBYTES (sp)
			sd		a4, 8 * GP_REGBYTES (sp)
			sd		a5, 9 * GP_REGBYTES (sp)
			sd		a6, 10 * GP_REGBYTES (sp)
			sd		a2, 11 * GP_REGBYTES (sp)
			sd		ra, 12 * GP_REGBYTES (sp)
			mv		a2, a7
			call	notification_handler
			ld		t0, 0 * GP_REGBYTES (sp)
			ld		t1, 1 * GP_REGBYTES (sp)
			ld		t2, 2 * GP_REGBYTES (sp)
			ld		t3, 3 * GP_REGBYTES (sp)
			ld		t4, 4 * GP_REGBYTES (sp)
			ld		t5, 5 * GP_REGBYTES (sp)
			ld		t6, 6 * GP_REGBYTES (sp)
			ld		a3, 7 * GP_REGBYTES (sp)
			ld		a4, 8 * GP_REGBYTES (sp)
			ld		a5, 9 * GP_REGBYTES (sp)
			ld		a6, 10 * GP_REGBYTES (sp)
			ld		a2, 11 * GP_REGBYTES (sp)
			ld		ra, 12 * GP_REGBYTES (sp)
			addi	sp, sp, (13 + 4) * GP_REGBYTES
			li		a7, NOTIFY_RETURN
			ecall
		",
			options(noreturn)
		);
	}
}

#[export_name = "notification_handler"]
extern "C" fn notification_handler(typ: usize, value: usize, address: usize) {
	kernel::sys_log!("oh my {:x} {:x} {:x}", typ, value, address);
}

#[export_name = "main"]
fn main() {
	// FIXME move this to rtbegin
	unsafe { dux::init() };

	// Parse arguments
	let mut pci = None;
	let mut bars = [None; 6];

	rtbegin::args().for_each(|a| {
		kernel::dbg!(core::str::from_utf8(a).unwrap());
	});

	let ret = unsafe { kernel::io_set_notify_handler(notification_handler_entry) };
	assert_eq!(ret.status, 0, "failed to set notify handler");

	let ret = unsafe { kernel::sys_reserve_interrupt(0x20) };
	assert_eq!(ret.status, 0, "failed to reserve interrupt");
	let ret = unsafe { kernel::sys_reserve_interrupt(0x21) };
	assert_eq!(ret.status, 0, "failed to reserve interrupt");
	let ret = unsafe { kernel::sys_reserve_interrupt(0x22) };
	assert_eq!(ret.status, 0, "failed to reserve interrupt");
	let ret = unsafe { kernel::sys_reserve_interrupt(0x23) };
	assert_eq!(ret.status, 0, "failed to reserve interrupt");

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

	match pci {
		pci::Header::H0(h) => {
			h.interrupt_line.set(0x0);
			h.interrupt_pin.set(0x1);
		}
		_ => todo!(),
	}

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
	let mut device = virtio::pci::new_device(pci, &virt_bars[..], virtio_block::BlockDevice::new)
		.expect("failed to create device");

	// Add self to registry
	let name = "virtio_block";
	let ret = unsafe { kernel::sys_registry_add(name.as_ptr(), name.len(), usize::MAX) };
	assert_eq!(ret.status, 0, "failed to add self to registry");

	// Wait for & respond to requests
	loop {
		unsafe { kernel::io_wait(u64::MAX) };
		let rxq = dux::ipc::receive();
		let op = rxq.opcode.unwrap();

		let ratio = kernel::Page::SIZE / core::mem::size_of::<virtio_block::Sector>();
		let length = rxq.length / virtio_block::Sector::SIZE;
		let offset = rxq.offset * ratio as u64;

		match kernel::ipc::Op::try_from(op) {
			Ok(kernel::ipc::Op::Read) => {
				let data = unsafe {
					let data = rxq.data.unwrap().as_ptr().cast::<virtio_block::Sector>();
					core::slice::from_raw_parts_mut(data, length)
				};

				device.read(data, offset).expect("failed to read sectors");

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

				device.write(data, offset).expect("failed to write sectors");

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
