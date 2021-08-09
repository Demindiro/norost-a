//! SBI interface.
//!
//! # References
//!
//! [RISC-V Supervisor Binary Interface Specification][sbi]
//!
//! [sbi]: https://github.com/riscv/riscv-sbi-doc/blob/master/riscv-sbi.adoc

// TODO Never inline for now so that registers are properly preserved.
#[inline(never)]
pub fn console_putchar(c: u8) {
	// SAFETY: calling putchar is entirely safe.
	unsafe {
		asm!("ecall", in("a7") 0x1, in("a6") 0, in("a0") c);
	}
}

// TODO ditto
#[inline(never)]
pub fn set_timer(value: u64) {
	// SAFETY: calling  set_timer should be safe.
	unsafe {
		asm!("ecall", in("a7") 0x0, in("a6") 0, in("a0") value);
	}
}
