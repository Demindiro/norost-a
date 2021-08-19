//! # UART driver
//!
//! All this driver does is buffer UART input and send received data over it. It is meant to be
//! used by one task only.
//!
//! The driver does not add itself to the registry! This must be done by the "parent" task.

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

/// The base address of the UART.
const ADDRESS: *mut u8 = 0x1000_0000 as *mut _;

/// Write buffer for data read.
///
/// 4 KiB should be quite enough.
static mut BUFFER: [u8; 1 << 12] = [0; 1 << 12];

// We spin it right round baby right round
/// The last index of data read from UART.
static mut NEW_INDEX: u16 = 0;

/// The last index of data read from the buffer
static mut USED_INDEX: u16 = 0;

/// Map & initialize a new UART interface at the given physical address.
///
/// The address is a PPN! The offset bits are not included. Similarly, the size refers to the
/// amount of pages, not bytes!
///
/// # Safety
///
/// The address must point to a valid UART device and is not already in use.
///
/// This function may only be called once.
#[must_use]
pub unsafe fn init(address: usize, size: usize) {
	// Map the device.
	let ret = kernel::sys_direct_alloc(ADDRESS.cast(), address, size, 0b011);
	assert_eq!(ret.status, 0, "mapping UART failed");

	// Initialize the device
	// Copied from https://wiki.osdev.org/Serial_Ports
	/*
	ADDRESS.add(3).write(0x80); // Enable DLAB (set baud rate divisor)
	ADDRESS.add(0).write(0x03); // Set divisor to 3 (lo byte) 38400 baud
	ADDRESS.add(1).write(0x00); //                  (hi byte)
	ADDRESS.add(3).write(0x03); // 8 bits, no parity, one stop bit
	ADDRESS.add(2).write(0xc7); // Enable FIFO, clear them, with 14-byte threshold
	ADDRESS.add(4).write(0x0b); // IRQs enabled, RTS/DSR set
	ADDRESS.add(4).write(0x1e); // Set in loopback mode, test the serial chip
	ADDRESS.add(0).write(0xae); // Test serial chip (send byte 0xAE and check if serial returns same byte)
	ADDRESS.add(3).write(0x80); // Enable DLAB (set baud rate divisor)
	*/
}

/// Enable / disable data available interrupts
pub fn interrupt_data_available(enable: bool) {
	unsafe {
		let m = ptr::read_volatile(ADDRESS.add(1));
		let m = (m & !(1 << 0)) | (u8::from(enable) << 0);
		ptr::write_volatile(ADDRESS.add(1), m);
	}
}

/// Enable / disable transmitter empty interrupts
#[allow(dead_code)]
pub fn interrupt_transmitter_empty(enable: bool) {
	unsafe {
		let m = ptr::read_volatile(ADDRESS.add(1));
		let m = (m & !(1 << 1)) | (u8::from(enable) << 1);
		ptr::write_volatile(ADDRESS.add(1), m);
	}
}

/// Check if any data to read is available.
#[must_use]
pub fn data_available() -> bool {
	unsafe { ptr::read_volatile(ADDRESS.add(5)) & 0x1 > 0 }
}

/// Check if it is possible to transmit data, i.e. the queue isn't full.
#[must_use]
pub fn can_transmit() -> bool {
	unsafe { ptr::read_volatile(ADDRESS.add(5)) & 0x20 > 0 }
}

/// Read a single byte.
#[must_use]
pub fn read() -> Option<u8> {
	data_available().then(|| unsafe { ptr::read_volatile(ADDRESS) })
}

/// Write a single byte.
#[must_use]
pub fn write(byte: u8) -> bool {
	let tr = can_transmit();
	tr.then(|| unsafe { ptr::write_volatile(ADDRESS, byte) });
	tr
}

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
	match (typ, value, address) {
		(0x0, intr, usize::MAX) if intr == 0xa => unsafe {
			let full_index = USED_INDEX.wrapping_add(BUFFER.len().try_into().unwrap());
			while let Some(c) = read() {
				if NEW_INDEX == full_index {
					// Disable data available interrupts for now, as we can't read more data anyways.
					interrupt_data_available(false);
					break;
				}
				BUFFER[usize::from(NEW_INDEX)] = c;
				NEW_INDEX = NEW_INDEX.wrapping_add(1);
			}
		},
		_ => (),
	}
}

#[export_name = "main"]
fn main() {
	// FIXME move this to rtbegin
	unsafe { dux::init() };

	let mut args = rtbegin::args();
	let arg = args.next().unwrap();
	let mut addr = args.next().unwrap();
	let mut size = args.next().unwrap();
	args.next().ok_or(()).unwrap_err();

	assert_eq!(arg, b"--reg");
	let addr = usize::from_str_radix(core::str::from_utf8(addr).unwrap(), 16).unwrap();
	let size = usize::from_str_radix(core::str::from_utf8(size).unwrap(), 16).unwrap();

	// Set up the notification handler _now_.
	let ret = unsafe { kernel::io_set_notify_handler(notification_handler_entry) };
	assert_eq!(ret.status, 0, "failed to set notify handler");

	// Setup the UART device.
	unsafe {
		init(
			addr / kernel::Page::SIZE,
			(size + kernel::Page::MASK) / kernel::Page::SIZE,
		)
	};

	// Reserve the UART interrupt.
	let ret = unsafe { kernel::sys_reserve_interrupt(0xa) };
	assert_eq!(ret.status, 0, "failed to reserve interrupt");

	// Enable UART data available interrupts.
	interrupt_data_available(true);

	// Wait for & respond to requests
	loop {
		let rxq = dux::ipc::receive();
		let op = rxq.opcode.unwrap();
		match kernel::ipc::Op::try_from(op) {
			Ok(kernel::ipc::Op::Read) => {
				// Figure out object to read.
				let data = unsafe {
					core::slice::from_raw_parts_mut(rxq.data.unwrap().as_ptr().cast(), rxq.length)
				};
				let path = rxq.name.map(|name| unsafe {
					core::slice::from_raw_parts(name.cast::<u8>().as_ptr(), rxq.name_len.into())
				});

				let mut length = 0;

				unsafe {
					// Wait until data is available
					// TODO this blocks writes from other tasks.
					while USED_INDEX == NEW_INDEX {
						kernel::io_wait(u64::MAX);
					}

					while USED_INDEX != NEW_INDEX && length < data.len() {
						data[length] = BUFFER[usize::from(USED_INDEX) & (BUFFER.len() - 1)];
						// Workaround QEMU sillyness
						if data[length] == b'\r' {
							data[length] = b'\n';
						}
						USED_INDEX = USED_INDEX.wrapping_add(1);
						length += 1;
					}

					// Re-enable UART data available interrupts if it was disabled.
					interrupt_data_available(true);
				}

				// Send completion event
				*dux::ipc::transmit() = kernel::ipc::Packet {
					uuid: kernel::ipc::UUID::from(0x09090909090555577777),
					opcode: Some(kernel::ipc::Op::Read.into()),
					name: None,
					name_len: 0,
					flags: 0,
					id: 0,
					address: rxq.address,
					data: None,
					length,
					offset: 0,
				};
			}
			Ok(kernel::ipc::Op::Write) => {
				// Figure out object to write to.
				let data = unsafe {
					core::slice::from_raw_parts(rxq.data.unwrap().as_ptr().cast(), rxq.length)
				};
				let path = rxq.name.map(|name| unsafe {
					core::slice::from_raw_parts(name.cast::<u8>().as_ptr(), rxq.name_len.into())
				});

				// Write data
				let mut len = 0;
				while len < data.len() {
					while !write(data[len]) {
						interrupt_transmitter_empty(true);
						unsafe { kernel::io_wait(u64::MAX) };
					}
					interrupt_transmitter_empty(false);
					len += 1;
				}

				// Confirm reception.
				*dux::ipc::transmit() = kernel::ipc::Packet {
					uuid: kernel::ipc::UUID::from(0x10101010101010),
					opcode: Some(kernel::ipc::Op::Write.into()),
					name: None,
					name_len: 0,
					flags: 0,
					id: 0,
					address: rxq.address,
					data: None,
					length: len,
					offset: 0,
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
