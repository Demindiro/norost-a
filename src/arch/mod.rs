#[cfg(any(target_arch = "riscv64"))]
mod riscv;
#[cfg(target_arch = "riscv64")]
pub use riscv::rv64 as riscv64;

use crate::log;
use core::ptr;

/// A wrappers that allows inspecting the capabilities of the current CPU
#[cfg(target_arch = "riscv64")]
pub struct Capabilities(riscv64::MISA);

impl Capabilities {
	/// Creates a new wrapper around whatever structures that need to be accessed to
	/// get the CPU's capabilities.
	pub fn new() -> Self {
		#[cfg(target_arch = "riscv64")]
		Capabilities(riscv64::MISA::new())
	}

	/// Logs the capabilities of the current CPU
	pub fn log(&self) {
		self.0.log()
	}
}

extern {
	#[link_name = "__stack_pointer"]
	static STACK_BASE: *const u8;
}
extern {
	#[link_name = "__text"]
	static TEXT_START: *const u8;
}
extern {
	#[link_name = "__etext"]
	static TEXT_END: *const u8;
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
	F: Fn(*const u8, *const u8)
{
	// TODO the current method is a very ugly way to get the backtrace.
	// While simply checking whether a stack value happens to be an address
	// to somewhere in .text, it's not reliable since some values may just
	// coincidence (e.g. array of function pointers, random integer with
	// value 0x8000_4394 etc)	
	// We should be able to scan the ELF file for a section with stack information
	// per function, though if that section isn't there we're probably out of luck...

	// SAFETY: extern static is defined and valid
	//let (stack_base, text_start, text_end); = unsafe {
	let (stack_base, text_start, text_end): (usize, usize, usize);
	unsafe {
		// TODO figure out why the extern statics produce garbage
		//(STACK_BASE as *const usize, TEXT_START as usize, TEXT_END as usize)
		asm!("
			la	t0, __stack_pointer
			la	t1, __text
			la	t2, __etext
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
