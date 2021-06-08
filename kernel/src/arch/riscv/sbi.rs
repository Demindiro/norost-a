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

pub fn hart_id() -> usize {
	// TODO hart ids need to be cached as there is no SBI method or S-mode
	// instruction to get them.
	0
}

pub fn hart_count() -> usize {
	// TODO hart count need to be cached as there is no SBI method or S-mode
	// instruction to get them.
	1
}
