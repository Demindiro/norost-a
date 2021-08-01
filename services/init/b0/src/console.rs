//! # Console I/O
//!
//! The current I/O reads from & writes to to UART.
//!
//! This should eventually be moved to a separate program as it is hardware-dependent.

use crate::uart;
use core::fmt;
use core::ptr::NonNull;

/// A console device to read from & write to.
pub struct Console {
	/// The UART device to send & read data over.
	pub uart: uart::UART,
}

impl Console {
	pub unsafe fn new(address: NonNull<u8>) -> Self {
		Self {
			uart: uart::UART::new(address),
		}
	}
}

impl fmt::Write for Console {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		for b in s.bytes() {
			self.uart.write(b).map_err(|_| fmt::Error)?
		}
		Ok(())
	}
}
