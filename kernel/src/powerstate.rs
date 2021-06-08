/// Halts the CPU until an interrupt is received
#[inline]
pub fn halt() {
	unsafe {
		asm!("wfi");
	}
}

/// Shuts the CPU down
#[inline]
pub fn shutdown() -> ! {
	log!("Shutting down");
	unsafe {
		asm!("ecall",
			 in("a7") 0x53525354, // SRST extension
			 in("a0") 0, // Shutdown function
			 in("a1") 0, // Reason
		);
	}
	log!("Failed to shut down!");
	log!("Entering halt loop");
	loop {
		halt();
	}
}
