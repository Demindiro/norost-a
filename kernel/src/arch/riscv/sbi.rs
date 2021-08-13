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
		// TODO idk why the hell the SBI is unable to handle full 64 bit integers.
		// 31 bits allows timeouts of about 30 minutes, so it should be fine for now.
		let value = value & 0x7fff_ffff;
		asm!("ecall", in("a7") 0x0, in("a6") 0, in("a0") value, in("a1") value);
	}
}
