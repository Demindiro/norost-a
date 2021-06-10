#[cfg(any(target_arch = "riscv64", target_arch = "riscv32"))]
pub mod riscv;
#[cfg(target_arch = "riscv64")]
use riscv::rv64 as riscv64;

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

use crate::{log, task, util};
use core::{mem, ptr};

extern "C" {
	/// Saves the registers of the given task and begins running the next task.
	///
	/// ## Safety
	///
	/// `pc` in the `registers` must be valid.
	pub fn trap_next_task(task: crate::task::Task) -> !;

	/// Begins running the given task.
	pub fn trap_start_task(task: crate::task::Task) -> !;
}

/// Initialize arch-specific structures, such as the interrupt table
pub fn init() {
	#[cfg(any(target_arch = "riscv64", target_arch = "riscv32"))]
	riscv::init();
	#[cfg(not(any(target_arch = "riscv64", target_arch = "riscv32")))]
	compile_error!("No arch init function defined");
}

/// A structure that encodes the JEDEC in a somewhat sensible way
pub struct JEDEC {
	/// The amount of continuation codes.
	continuation: usize,
	/// The stop (i.e. termination) code.
	stop: u8,
}

/// A wrapper that allows inspecting the capabilities of the current CPU
pub trait Capabilities {
	/// Creates a new wrapper around whatever structures that need to be accessed to
	/// get the CPU's capabilities.
	fn new() -> Self;

	/// Logs the capabilities of the current CPU
	fn log(&self);
}

/// A wrapper to get CPU-specific information, such as the vendor
pub trait ID {
	/// Creates a wrapper around the structures needed for accessing CPU-specific information.
	fn new() -> Self;

	/// Gets the JEDEC ID of this CPU.
	fn jedec(&self) -> JEDEC;

	/// Gets the architecture ID.
	fn arch(&self) -> usize;

	/// Gets the current hardware thread (i.e. hart)
	fn hart(&self) -> usize;

	/// Logs vendor-specific information.
	fn log(&self) {
		let jedec = self.jedec();
		log!("Vendor: {} {}", jedec.continuation, jedec.stop);
		log!("Architecture: {}", self.arch());
		log!("Current hart: {}", self.hart());
	}
}

const _PAGE_ALIGN_CHECK: usize = 0 - (PAGE_SIZE - core::alloc::Layout::new::<Page>().align());

pub fn capabilities() -> impl Capabilities {
	#[cfg(any(target_arch = "riscv64", target_arch = "riscv32"))]
	riscv64::MISA::new()
}

pub fn id() -> impl ID {
	#[cfg(any(target_arch = "riscv64", target_arch = "riscv32"))]
	riscv64::ID::new()
}

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

/// Returns the ID of the current hart.
pub fn hart_id() -> usize {
	riscv::sbi::hart_id()
}

/// Returns the total amount of harts on the system
#[cold]
pub fn hart_count() -> usize {
	riscv::sbi::hart_count()
}

#[must_use]
pub fn add_kernel_mapping<F: FnMut() -> crate::memory::PPN>(f: F, count: usize, rwx: RWX) -> core::ptr::NonNull<Page> {
	riscv::vms::Sv39::add_kernel_mapping(f, count, rwx)
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
