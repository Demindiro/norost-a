/// Halts the CPU until an interrupt is received
#[inline]
pub fn halt() {
	unsafe {
		asm!("wfi");
	}
}

/// Shuts the CPU down
// FIXME it doesn't work in QEMU (at least, it doesn't _exit_ Qemu, it does seem to get stuck
// though). Figure out how to make it work.
#[inline]
pub fn shutdown() {
	unsafe {
		crate::log::info(&["Shutting down"]);
		asm!("ecall",
			 in("a7") 0x53525354, // SRST extension
			 in("a0") 0, // Shutdown function
			 in("a1") 0, // Reason
		);
		crate::log::fatal(&["Failed to shut down!"]);
	}
}
