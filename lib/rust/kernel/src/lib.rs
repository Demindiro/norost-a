//! Low level bindings for the kernel API

#![no_std]
#![feature(asm)]

use core::ffi;
use core::fmt;

pub const IO_NONE: u8 = 0;
pub const IO_READ: u8 = 1;
pub const IO_WRITE: u8 = 2;

pub const PROT_READ: u8 = 0x1;
pub const PROT_WRITE: u8 = 0x2;
pub const PROT_EXEC: u8 = 0x4;

/// Structure returned by system calls.
#[repr(C)]
pub struct Return {
	pub status: usize,
	pub value: usize,
}

pub mod ipc {
	use super::Page;
	use core::num::NonZeroU8;
	use core::ptr::NonNull;

	/// Union of all possible data types that can be pointed to in a packet's `data` field.
	///
	/// All members are pointers.
	pub union Data {
		/// An address range with raw data to be read or to which data should be written.
		pub raw: *mut u8,
	}

	/// Structure used to communicate with other tasks.
	#[repr(C)]
	pub struct Packet {
		pub opcode: Option<NonZeroU8>,
		pub priority: i8,
		pub flags: u16,
		pub id: u32,
		pub address: usize,
		pub length: usize,
		pub data: Data,
	}

	#[repr(u8)]
	pub enum Op {
		Read = 1,
		Write = 2,
	}

	impl From<Op> for NonZeroU8 {
		fn from(op: Op) -> Self {
			NonZeroU8::new(match op {
				Op::Read => 1,
				Op::Write => 2,
			})
			.unwrap()
		}
	}

	#[repr(C)]
	pub struct FreePage {
		pub address: Option<NonNull<Page>>,
		pub count: usize,
	}
}

#[repr(C)]
pub struct TaskSpawnMapping {
	pub task_address: *mut Page,
	pub typ: u8,
	pub flags: u8,
	pub self_address: *mut Page,
}

#[macro_use]
#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
mod riscv;
#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
pub use riscv::*;

syscall!(io_wait, 0, flags: u8, time: u64);
syscall!(
	io_set_queues,
	1,
	transmit_queue: *mut ipc::Packet,
	transmit_size: usize,
	receive_queue: *mut ipc::Packet,
	receive_size: usize,
	free_pages: *mut ipc::FreePage,
	free_pages_size: usize
);

syscall!(mem_alloc, 3, address: *mut Page, size: usize, flags: u8);
syscall!(mem_dealloc, 4, address: *mut Page, size: usize);
syscall!(
	mem_physical_address,
	7,
	address: *const Page,
	store: *mut usize,
	count: usize
);

syscall!(
	task_spawn,
	11,
	mappings: *const TaskSpawnMapping,
	mappings_count: usize,
	program_counter: *const ffi::c_void,
	stack_pointer: *const ffi::c_void
);

syscall!(
	dev_dma_alloc,
	12,
	address: *mut Page,
	size: usize,
	flags: u8
);

syscall!(sys_platform_info, 13, address: *mut Page, max_count: usize);
syscall!(
	sys_direct_alloc,
	14,
	address: *mut Page,
	start_page: usize,
	count: usize,
	flags: u8
);
syscall!(sys_log, 15, string: *const u8, length: usize);

/// Interface for sending messages to the kernel log.
pub struct SysLog;

impl fmt::Write for SysLog {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		unsafe {
			let ret = sys_log(s as *const _ as *const _, s.len());
			(ret.status == 0).then(|| ()).ok_or(fmt::Error)
		}
	}
}

/// A macro that acts similar to println but sends output to the kernel log.
#[macro_export]
macro_rules! sys_log {
	($($arg:tt)*) => {{
		use core::fmt::Write;
		let _ = writeln!($crate::SysLog, $($arg)*);
	}};
}
