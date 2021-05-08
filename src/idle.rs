/// Halts the CPU until an interrupt is received
#[inline(always)]
pub fn halt() {
	unsafe {
		asm!("wfi");
	}
}
