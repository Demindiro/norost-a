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
    _todo: (),
}

#[repr(C)]
pub struct ServerRequestEntry {
    _todo: (),
}

#[repr(C)]
pub struct ServerCompletionEntry {
    _todo: (),
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

syscall!(mem_alloc, 3, address: *mut Page, size: usize, flags: u8);
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
