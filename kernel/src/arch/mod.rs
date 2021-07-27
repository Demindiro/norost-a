use crate::memory::ppn::*;
use core::convert::TryFrom;

#[cfg(any(target_arch = "riscv64", target_arch = "riscv32"))]
pub mod riscv;

/// The size of a single memory page.
#[cfg(any(target_arch = "riscv64", target_arch = "riscv32"))]
pub use riscv::PAGE_SIZE;

/// The ELF type of this architecture.
#[cfg(any(target_arch = "riscv64", target_arch = "riscv32"))]
pub use riscv::elf::MACHINE as ELF_MACHINE;

#[cfg(any(target_arch = "riscv64", target_arch = "riscv32"))]
pub use riscv::RegisterState;

#[cfg(any(target_arch = "riscv64", target_arch = "riscv32"))]
pub use riscv::Page;

/// A system to manage virtual to physical memory mappings.
#[cfg(target_arch = "riscv64")]
pub type VirtualMemorySystem = riscv::vms::Sv39;

#[cfg(target_arch = "riscv64")]
pub type RWX = riscv::vms::RWX;

/// All supported ELF flags.
// FIXME we need a way to detect individual features at compile time.
// Alternatively, we make this a static and use the MISA register.
#[cfg(any(target_arch = "riscv64", target_arch = "riscv32"))]
pub const ELF_FLAGS: u32 = riscv::elf::RVC | riscv::elf::FLOAT_ABI_DOUBLE;

/// A bitmask that covers the lower zeroed bits of an aligned page.
pub const PAGE_MASK: usize = PAGE_SIZE - 1;

/// The amount of bits that are zero due to page alignment.
pub const PAGE_BITS: usize = 12;

use core::ptr;

extern "C" {
	/// Saves the registers of the given task and begins running the next task.
	///
	/// ## Safety
	///
	/// `pc` in the `registers` must be valid.
	// Task _is_ FFI-safe you stupid fucking compiler.
	#[allow(improper_ctypes)]
	pub fn trap_next_task(task: crate::task::Task) -> !;

	/// Begins running the given task.
	// Task _is_ FFI-safe you stupid fucking compiler.
	#[allow(improper_ctypes)]
	pub fn trap_start_task(task: crate::task::Task) -> !;
}

/// Initialize arch-specific structures, such as the interrupt table
pub fn init() {
	#[cfg(any(target_arch = "riscv64", target_arch = "riscv32"))]
	riscv::init();
	#[cfg(not(any(target_arch = "riscv64", target_arch = "riscv32")))]
	compile_error!("No arch init function defined");
}

const _PAGE_ALIGN_CHECK: usize = 0 - (PAGE_SIZE - core::alloc::Layout::new::<Page>().align());

/// Returns `true` if an accurate backtrace can be returned, otherwise `false`.
pub fn is_backtrace_accurate() -> bool {
	false
}

/// Calls the given closure for each stack frame, giving a complete backtrace.
#[cold]
// inline(never) is important for correctness
#[inline(never)]
pub fn backtrace<F>(f: F)
where
	F: Fn(*const u8, *const u8),
{
	// TODO the current method is a very ugly way to get the backtrace.
	// While simply checking whether a stack value happens to be an address
	// to somewhere in .text, it's not reliable since some values may just
	// coincidence (e.g. array of function pointers, random integer with
	// value 0x8000_4394 etc)
	// We should be able to scan the ELF file for a section with stack information
	// per function, though if that section isn't there we're probably out of luck...

	// SAFETY: extern static is defined and valid
	let (stack_base, text_start, text_end): (usize, usize, usize);
	unsafe {
		// TODO figure out why extern statics produce garbage
		asm!("
			la	t0, _stack_pointer
			la	t1, _text_start
			la	t2, _text_end
		", out("t0") stack_base, out("t1") text_start, out("t2") text_end);
	};
	let mut sp: usize;
	// SAFETY: we're only reading the current stack pointer
	unsafe {
		// rust is really fucking dumb. Let me utilize the full power of `unsafe` damnit!
		asm!("mv	t0, sp", out("t0") sp);
	}
	while sp != stack_base {
		// SAFETY: sp is inside the stack
		let addr = unsafe { ptr::read_unaligned(sp as *const usize) };
		// SAFETY: extern static is defined and valid
		if text_start <= addr && addr < text_end {
			f(sp as _, addr as _);
		}
		sp += 4;
	}
}

#[inline]
pub fn set_supervisor_userpage_access(enable: bool) {
	let sum_bit = 1 << 18;
	let mut sstatus: usize;
	unsafe { asm!("csrr {0}, sstatus", out(reg) sstatus) };
	sstatus &= !sum_bit;
	sstatus |= sum_bit * (enable as usize);
	unsafe { asm!("csrw sstatus, {0}", in(reg) sstatus) };
}

/// A PPN of a certain type.
#[repr(u8)]
pub enum Map {
	Private(PPN) = 0b00,
	Direct(PPNDirect) = 0b01,
	Shared(SharedPPN) = 0b10,
	SharedLocked(SharedPPN) = 0b11,
}

impl Map {}

/// A range of PPNs of a certain type.
#[repr(u8)]
pub enum MapRange {
	Private(PPNRange) = 0b00,
	Direct(PPNDirectRange) = 0b01,
	Shared(SharedPPNRange) = 0b10,
	SharedLocked(SharedPPNRange) = 0b11,
}

impl MapRange {
	pub fn len(&self) -> usize {
		match self {
			Self::Private(p) => p.len(),
			Self::Direct(d) => d.len(),
			Self::Shared(s) | Self::SharedLocked(s) => s.len(),
		}
	}

	pub fn start(&self) -> PPNBox {
		match self {
			Self::Private(p) => p.start(),
			Self::Direct(d) => d.start(),
			Self::Shared(s) | Self::SharedLocked(s) => s.start(),
		}
	}

	pub fn pop_base(&mut self) -> Option<Map> {
		match self {
			Self::Private(p) => p.pop_base().map(Map::Private),
			Self::Direct(d) => d.pop_base().map(Map::Direct),
			Self::Shared(s) => s.pop_base().map(Map::Shared),
			Self::SharedLocked(s) => s.pop_base().map(Map::SharedLocked),
		}
	}

	pub fn forget_base(&mut self, count: usize) -> usize {
		match self {
			Self::Private(p) => p.forget_base(count),
			Self::Direct(d) => d.forget_base(count),
			Self::Shared(s) | Self::SharedLocked(s) => s.forget_base(count),
		}
	}
}
