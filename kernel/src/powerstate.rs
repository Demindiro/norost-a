/// Halts the CPU until an interrupt is received
#[inline]
pub fn halt() {
	unsafe {
		asm!("wfi");
	}
}
