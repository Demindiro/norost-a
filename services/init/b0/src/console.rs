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

	pub fn read(&mut self, buf: &mut [u8]) -> usize {
		let mut i = 0;
		while let Some(c) = self.uart.read() {
			if let Some(w) = buf.get_mut(i) {
				*w = c;
				i += 1;
			} else {
				break;
			}
		}
		i
	}

	pub fn write(&mut self, buf: &[u8]) -> usize {
		for (i, c) in buf.iter().copied().enumerate() {
			if self.uart.write(c).is_err() {
				return i;
			}
		}
		return buf.len();
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
