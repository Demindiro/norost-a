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
#[allow(dead_code)]
pub fn set_timer(value: u64) {
	// SAFETY: calling  set_timer should be safe.
	unsafe {
		/* TODO clobber_abi pls
		let mask = 1 << 5; // stie
		asm!("csrc sie, {0}", in(reg) mask);
		asm!("ecall", in("a7") 0x54494d45, in("a6") 0, in("a0") value, clobber("C"));
		asm!("csrs sie, {0}", in(reg) mask);
		*/
		asm!("
			li		s2, 1 << 5
			csrc	sie, s2
			ecall
			csrs	sie, s2
		", in("a7") 0x54494d45, in("a6") 0, in("a0") value, lateout("s2") _);
	}
}
