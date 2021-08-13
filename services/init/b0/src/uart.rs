//! # UART wrapper

use core::fmt;
use core::ptr::{self, NonNull};

/// An UART interface
pub struct UART {
	/// The base address of the port
	address: NonNull<u8>,
}

impl UART {
	/// Initialize a new UART interface.
	///
	/// # Safety
	///
	/// The address must point to a valid UART device and is not already in use.
	#[must_use]
	pub unsafe fn new(address: NonNull<u8>) -> Self {
		let a = address.as_ptr();
		// Copied from https://wiki.osdev.org/Serial_Ports
		//a.add(0).write(0x00); // Enable DLAB (set baud rate divisor)
		a.add(1).write(0x01); // Enable data available interrupts
					  /*
					  a.add(3).write(0x80); // Enable DLAB (set baud rate divisor)
					  a.add(0).write(0x03); // Set divisor to 3 (lo byte) 38400 baud
					  a.add(1).write(0x00); //                  (hi byte)
					  a.add(3).write(0x03); // 8 bits, no parity, one stop bit
					  a.add(2).write(0xc7); // Enable FIFO, clear them, with 14-byte threshold
					  a.add(4).write(0x0b); // IRQs enabled, RTS/DSR set
					  a.add(4).write(0x1e); // Set in loopback mode, test the serial chip
					  a.add(0).write(0xae); // Test serial chip (send byte 0xAE and check if serial returns same byte)
					  */
		Self { address }
	}

	/// Check if any data to read is available.
	#[must_use]
	pub fn data_available(&self) -> bool {
		unsafe { ptr::read_volatile(self.address.as_ptr().add(5)) & 0x1 > 0 }
	}

	/// Check if it is possible to transmit data, i.e. the queue isn't full.
	#[must_use]
	pub fn can_transmit(&self) -> bool {
		unsafe { ptr::read_volatile(self.address.as_ptr().add(5)) & 0x20 > 0 }
	}

	/// Read a single byte.
	#[must_use]
	pub fn read(&mut self) -> Option<u8> {
		self.data_available()
			.then(|| unsafe { ptr::read_volatile(self.address.as_ptr()) })
	}

	/// Write a single byte.
	#[must_use]
	pub fn write(&mut self, byte: u8) -> Result<(), BufferFull> {
		self.can_transmit()
			.then(|| unsafe { ptr::write_volatile(self.address.as_ptr(), byte) })
			.ok_or(BufferFull)
	}
}

pub struct BufferFull;

impl fmt::Debug for BufferFull {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		"buffer full".fmt(f)
	}
}

impl fmt::Display for BufferFull {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Debug::fmt(self, f)
	}
}
