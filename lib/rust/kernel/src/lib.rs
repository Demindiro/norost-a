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

pub mod ipc {
	use super::*;
	use core::convert::TryFrom;
	use core::mem;
	use core::num::NonZeroU8;
	use core::ptr;
	use core::ptr::NonNull;

	/// An UUID used to uniquely identify objects.
	#[repr(C)]
	#[derive(Clone, Copy, Default)]
	pub struct UUID {
		x: u64,
		y: u64,
	}

	impl From<u128> for UUID {
		fn from(uuid: u128) -> Self {
			Self {
				x: uuid as u64,
				y: (uuid >> 64) as u64,
			}
		}
	}

	impl From<UUID> for u128 {
		fn from(uuid: UUID) -> Self {
			uuid.x as u128 | ((uuid.y as u128) << 64)
		}
	}

	impl fmt::Debug for UUID {
		fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
			write!(f, concat!(stringify!(UUID), "({})"), self)
		}
	}

	impl fmt::Display for UUID {
		fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
			let n = u128::from(*self);
			write!(f, "{:02x}{:02x}", n as u8, (n >> 8) as u8)?;
			for i in 1..8 {
				"-".fmt(f)?;
				write!(
					f,
					"{:02x}{:02x}",
					(n >> (16 * i)) as u8,
					(n >> (24 * i)) as u8
				)?;
			}
			Ok(())
		}
	}

	/// Structure used to communicate with other tasks.
	#[derive(Debug, Default)]
	#[repr(C)]
	pub struct Packet {
		pub uuid: UUID,
		pub data: Option<NonNull<Page>>,
		pub name: Option<NonNull<Page>>,
		pub offset: u64,
		pub length: usize,
		pub address: usize,
		pub flags: u16,
		pub name_len: u16,
		pub id: u8,
		pub opcode: Option<NonZeroU8>,
	}

	#[derive(Debug)]
	#[repr(u8)]
	pub enum Op {
		Read = 1,
		Write = 2,
		Info = 3,
		List = 4,
		MapRead = 5,
		MapWrite = 6,
		MapReadWrite = 7,
		MapExec = 8,
		MapReadExec = 9,
		MapReadCow = 10,
		MapExecCow = 11,
		MapReadExecCow = 12,
	}

	impl From<Op> for NonZeroU8 {
		fn from(op: Op) -> Self {
			// SAFETY: we defined values for each of the variants.
			NonZeroU8::new(op as u8).unwrap()
		}
	}

	#[derive(Debug)]
	pub struct UnknownOp;

	impl TryFrom<NonZeroU8> for Op {
		type Error = UnknownOp;

		fn try_from(op: NonZeroU8) -> Result<Self, Self::Error> {
			let variant_count = mem::variant_count::<Self>();
			if usize::from(op.get()) <= variant_count {
				// SAFETY: there are no gaps in the variant list nor is the value out of bounds.
				Ok(unsafe { mem::transmute(op) })
			} else {
				Err(UnknownOp)
			}
		}
	}

	#[repr(C)]
	pub struct FreePage {
		pub address: Option<NonNull<Page>>,
		pub count: usize,
	}

	impl fmt::Debug for FreePage {
		fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
			f.debug_struct(stringify!(FreePage))
				.field(
					"address",
					&self
						.address
						.map(|p| p.as_ptr())
						.unwrap_or_else(ptr::null_mut),
				)
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

syscall!(io_wait, 0, flags: u8, time: u64);
syscall!(
	io_set_queues,
	1,
	packets: *mut ipc::Packet,
	mask_bits: u8,
	free_pages: *mut ipc::FreePage,
	free_pages_size: usize
);
syscall!(
	io_set_notify_handler,
	2,
	function: notification::Handler
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
	sys_set_interrupt_controller,
	8,
	ppn: usize,
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
