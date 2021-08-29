//! Low level bindings for the kernel API

#![no_std]
#![feature(asm)]
#![feature(variant_count)]

use core::convert::TryFrom;
use core::ffi;
use core::fmt;

pub const IO_NONE: u8 = 0;
pub const IO_READ: u8 = 1;
pub const IO_WRITE: u8 = 2;

pub const PROT_READ: u8 = 0x1;
pub const PROT_WRITE: u8 = 0x2;
pub const PROT_EXEC: u8 = 0x4;
pub const PROT_READ_WRITE: u8 = PROT_READ | PROT_WRITE;
pub const PROT_READ_EXEC: u8 = PROT_READ | PROT_EXEC;
pub const PROT_READ_WRITE_EXEC: u8 = PROT_READ | PROT_WRITE | PROT_EXEC;

/// Structure returned by system calls.
#[repr(C)]
pub struct Return {
	pub status: usize,
	pub value: usize,
}

impl Return {
	pub const OK: usize = 0;
	pub const INVALID_CALL: usize = 1;
	pub const NULL_ARGUMENT: usize = 2;
	pub const MEMORY_OVERLAP: usize = 3;
	pub const MEMORY_UNAVAILABLE: usize = 4;
	pub const MEMORY_LOCKED: usize = 5;
	pub const MEMORY_NOT_ALLOCATED: usize = 6;
	pub const MEMORY_INVALID_PROTECTION_FLAGS: usize = 7;
	pub const BAD_ALIGNMENT: usize = 8;
	pub const NOT_FOUND: usize = 9;
	pub const TOO_LONG: usize = 10;
	pub const OCCUPIED: usize = 11;
}

pub mod ipc {
	use super::*;
	use core::convert::TryFrom;
	use core::mem;
	use core::num::NonZeroU8;
	use core::ptr;
	use core::ptr::NonNull;
	use core::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

	/// Structure to uniquely identify a task.
	#[derive(Clone, Copy)]
	#[repr(transparent)]
	pub struct TaskID(u32);

	impl TaskID {
		pub const INVALID: Self = Self(u32::MAX);
		pub const KERNEL: Self = Self(u32::MAX);

		pub const fn new(id: u32) -> Self {
			Self(id)
		}
	}

	impl Default for TaskID {
		fn default() -> Self {
			Self::INVALID
		}
	}

	impl From<u32> for TaskID {
		fn from(n: u32) -> Self {
			Self(n)
		}
	}

	impl From<TaskID> for u32 {
		fn from(id: TaskID) -> Self {
			id.0
		}
	}

	impl fmt::Debug for TaskID {
		fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
			"TaskID(".fmt(f)?;
			self.0.fmt(f)?;
			")".fmt(f)
		}
	}

	/// Structure used to communicate with other tasks.
	#[derive(Clone, Debug, Default)]
	#[repr(C)]
	pub struct Packet {
		pub data: Option<NonNull<Page>>,
		pub name: Option<NonNull<Page>>,
		pub data_offset: u64,
		pub data_length: u32,
		pub address: TaskID,
		pub flags_user: u16,
		pub flags_kernel: u16,
		pub name_length: u16,
		pub id: u16,
	}

	impl Packet {
		pub const ZEROED: Self = Self {
			data: None,
			name: None,
			data_offset: 0,
			data_length: 0,
			address: TaskID::new(0),
			flags_user: 0,
			flags_kernel: 0,
			name_length: 0,
			id: 0,
		};
	}

	#[repr(C)]
	pub struct FreeRange {
		pub address: AtomicPtr<Page>,
		pub count: AtomicUsize,
	}

	impl FreeRange {}

	impl fmt::Debug for FreeRange {
		fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
			f.debug_struct(stringify!(FreeRange))
				.field("address", &self.address.load(Ordering::Relaxed))
				.field("count", &self.count)
				.finish()
		}
	}
}

pub mod notification {
	/// The handler function type
	pub type Handler = extern "C" fn();
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

syscall!(saveall io_wait, 0, time: u64);
syscall!(io_set_queues, 1, base: *mut (), mask_bits: u8);
syscall!(io_set_notify_handler, 2, function: notification::Handler);

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
	sys_set_interrupt_controller,
	8,
	ppn: usize,
	count: usize,
	max_devices: u16
);

syscall!(io_notify_return, 9);

syscall!(sys_reserve_interrupt, 10, interrupt: usize);

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

syscall!(
	sys_registry_add,
	16,
	name: *const u8,
	name_length: usize,
	address: usize
);
syscall!(sys_registry_get, 17, name: *const u8, name_length: usize);

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

/// Representation of a Physical Page Number.
///
/// A Physical Page Number is a physical pointer to a page without the offset bits. e.g.
/// the physical address `0x123000` on RISC-V would translate to a PPN of `0x123`
#[repr(transparent)]
pub struct PPN(usize);

impl From<usize> for PPN {
	fn from(ppn: usize) -> Self {
		Self(ppn)
	}
}

impl From<PPN> for usize {
	fn from(ppn: PPN) -> Self {
		ppn.0
	}
}

#[derive(Debug)]
pub struct OutOfRange;

impl fmt::Display for OutOfRange {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		"out of range".fmt(f)
	}
}

#[derive(Debug)]
pub struct BadAlignment;

impl fmt::Display for BadAlignment {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		"bad alignment".fmt(f)
	}
}

/// Representation of a physical address.
///
/// A physical address may not be able to fit a PPN.
#[repr(transparent)]
pub struct PhysicalAddress(usize);

impl TryFrom<PPN> for PhysicalAddress {
	type Error = OutOfRange;

	fn try_from(ppn: PPN) -> Result<Self, Self::Error> {
		ppn.0
			.checked_shl(Page::OFFSET_BITS.into())
			.map(Self)
			.ok_or(OutOfRange)
	}
}

impl TryFrom<PhysicalAddress> for PPN {
	type Error = BadAlignment;

	fn try_from(pa: PhysicalAddress) -> Result<Self, Self::Error> {
		(pa.0 & Page::MASK == 0)
			.then(|| Self(pa.0 >> Page::OFFSET_BITS))
			.ok_or(BadAlignment)
	}
}

impl From<usize> for PhysicalAddress {
	fn from(pa: usize) -> Self {
		Self(pa)
	}
}

impl From<PhysicalAddress> for usize {
	fn from(pa: PhysicalAddress) -> Self {
		pa.0
	}
}

// Shamelessly copied from stdlib.
#[macro_export]
macro_rules! dbg {
    () => {
        $crate::sys_log!("[{}:{}]", file!(), line!());
    };
    ($val:expr $(,)?) => {
        // Use of `match` here is intentional because it affects the lifetimes
        // of temporaries - https://stackoverflow.com/a/48732525/1063961
        match $val {
            tmp => {
                $crate::sys_log!("[{}:{}] {} = {:#?}",
                    file!(), line!(), stringify!($val), &tmp);
                tmp
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        ($($crate::dbg!($val)),+,)
    };
}
