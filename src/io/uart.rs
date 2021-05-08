use super::*;
use core::{num, ptr};
use core::convert::TryInto;

/// The default UART address. Used in [`UART::new()`](UART::new)
const BASE: *mut u8 = 0x10000000 as _;
const QUEUE: isize = 0x0;
const LINESTAT: isize = 0x5;
const STATUS_RX: u8 = 0x01;
const STATUS_TX: u8 = 0x20;

// TODO initialize the UART properly
// It works with QEMU right now, but it won't necessarily work on real hardware
unsafe fn init(address: *mut u8) {}

/// Writes a single character. Returns `false` if no data has been written, otherwise `true`.
#[must_use]
unsafe fn putc(address: *mut u8, character: u8) -> bool {
	(ptr::read_volatile(address.offset(LINESTAT)) & STATUS_TX != 0)
		.then(|| ptr::write_volatile(address.offset(QUEUE), character))
		.is_some()
}

/// Reads a single character. Returns `None` if there is no data to read.
#[must_use]
unsafe fn getc(address: *const u8) -> Option<u8> {
	(ptr::read_volatile(address.offset(LINESTAT)) & STATUS_RX != 0)
		.then(|| ptr::read_volatile(address.offset(QUEUE)))
}

/// A safe wrapper for UART operations
pub struct UART {
	address: *mut u8,
}

impl UART {
	/// Creates a new UART wrapper using the default address and initializes it.
	///
	/// ## Safety
	///
	/// This function is called only once.
	pub unsafe fn new() -> Self {
		let address = BASE;
		init(address);
		Self { address }
	}
}

impl Device for UART {
	fn write(&mut self, data: &[u8]) -> Result<NonZeroUsize, WriteError> {
		if data.len() > 0 {
			let mut i = 0;
			// SAFETY: This instance's address is valid
			while unsafe { putc(self.address, data[i]) } {
				i += 1;
				if data.len() <= i {
					break;
				}
			}
			i.try_into().ok().ok_or(WriteError::Full)
		} else {
			Err(WriteError::DataIsZeroSized)
		}
	}

	fn read(&mut self, buffer: &mut [u8]) -> Result<NonZeroUsize, ReadError> {
		if buffer.len() > 0 {
			let mut i = 0;
			// SAFETY: This instance's address is valid
			while let Some(b) = unsafe { getc(self.address) } {
				buffer[i] = b;
				i += 1;
				if buffer.len() <= i {
					break;
				}
			}
			i.try_into().ok().ok_or(ReadError::Empty)
		} else {
			Err(ReadError::BufferIsZeroSized)
		}
	}

	fn close(self) {
		// TODO
	}
}
