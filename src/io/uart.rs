//! I/O for uart16550

use core::ptr;

/// The default UART address. Used in [`UART::new()`](UART::new)
const BASE: *mut u8 = 0x10000000 as _;
const QUEUE: isize = 0x0;
const RBR: isize = 0x0;
const IER: isize = 0x1;
const FCR: isize = 0x2;
const LCR: isize = 0x3;
const LINESTAT: isize = 0x5;
const STATUS_RX: u8 = 0x01;
const STATUS_TX: u8 = 0x20;

unsafe fn init(address: *mut u8) {
	// Reset UART line
	ptr::write_volatile(address.offset(LCR), 0b11);
	// Enable FIFO
	ptr::write_volatile(address.offset(FCR), 1);
	// Enable interrupts
	ptr::write_volatile(address.offset(IER), 1);
}

/// Writes a single character
unsafe fn putc(address: *mut u8, character: u8) {
	// Wait until the device is ready to receive another character
	while ptr::read_volatile(address.offset(LINESTAT)) & STATUS_TX == 0 {}
	ptr::write_volatile(address.offset(QUEUE), character);
}

/// Reads a single character. Returns `None` if there is no data to read.
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

	/// Writes a string
	pub fn write_str(&mut self, string: &str) {
		for b in string.bytes() {
			// SAFETY: This UART is valid
			unsafe {
				putc(self.address, b);
			}
		}
	}
}
