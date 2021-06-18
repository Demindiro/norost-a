//! Low level bindings for the kernel API

#![no_std]
#![feature(asm)]

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

#[repr(C)]
pub struct ClientRequestEntry {
	pub opcode: u8,
	pub priority: i8,
	pub flags: u16,
	pub file_handle: u32,
	pub offset: usize,
	pub data_page: *const u8,
	pub length: usize,
	pub userdata: usize,
}

#[repr(C)]
pub struct ClientCompletionEntry {
	_todo: ()
}

#[repr(C)]
pub struct ServerRequestEntry {
	_todo: ()
}

#[repr(C)]
pub struct ServerCompletionEntry {
	_todo: ()
}

#[macro_use]
#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
mod riscv;
#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
pub use riscv::*;

syscall!(io_wait, 0, flags: u8, time: u64);

syscall!(mem_alloc, 3, address: *mut Page, size: usize, flags: u8);
syscall!(mem_physical_address, 7, address: *const Page, store: *mut usize, count: usize);

syscall!(dev_reserve, 10, id: usize, reg: *mut Page, ranges: *const *mut Page, ranges_count: usize);
syscall!(dev_mmio_map, 11, id: usize, size: usize, flags: u8);
syscall!(dev_dma_alloc, 12, address: *mut Page, size: usize, flags: u8);

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
