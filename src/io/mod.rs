pub mod uart;

use core::num::NonZeroUsize;

/// A trait that describes common I/O errors that can occur during writing
pub enum WriteError {
	/// The device has been closed and thus does no longer accept data
	Closed,
	/// The device's write buffer is full and this can not accept any data
	Full,
	/// The buffer is zero-sized, thus there is no room to read anything
	DataIsZeroSized,
}

/// A trait that describes common I/O errors that can occur during writing
pub enum ReadError {
	/// The device has been closed and thus can no longer return data
	Closed,
	/// There is no data to read at the moment
	Empty,
	/// The buffer is zero-sized, thus there is nothing to be written
	BufferIsZeroSized,
}

/// Trait that provides an universal interface for I/O
///
/// All the methods take `&mut self` to ensure the caller has exclusive access to the device,
/// which should allow some more optimizations.
pub trait Device {
	/// Attempts to write the given slice of bytes.
	///
	/// ## Returns
	///
	/// `Ok(NonZeroUsize)` if any data was successfully written, where `NonZeroUsize` denotes
	/// the total amount of bytes written.
	/// `Err(WriteError)` if the device failed to write the data for whatever reason.
	fn write(&mut self, data: &[u8]) -> Result<NonZeroUsize, WriteError>;

	/// Attempts to write the givne `str` as an `[u8]` slice.
	///
	/// ## Returns
	///
	/// `Ok(NonZeroUsize)` if any data was successfully written, where `NonZeroUsize` denotes
	/// the total amount of **bytes** written.
	/// `Err(WriteError)` if the device failed to write the data for whatever reason.
	fn write_str(&mut self, string: &str) -> Result<NonZeroUsize, WriteError> {
		self.write(string.as_bytes())
	}

	/// Attempts to read an amount of bytes into the given slice.
	///
	/// ## Returns
	///
	/// `Ok(NonZeroUsize)` if any data was successfully read, where `NonZeroUsize` denotes the
	/// total amount of bytes read.
	/// `Err(ReadError)` if the device failed to read any data for whatever reason.
	///
	/// ## Notes
	///
	/// This function returns a `NonZeroUsize` to keep the size of the return type limited to 16
	/// bytes on 64 bit platforms.
	fn read(&mut self, data: &mut [u8]) -> Result<NonZeroUsize, ReadError>;

	/// Closes this interface to the device. This method may never fail.
	///
	/// ## Notes
	///
	/// While certain standards such as POSIX do define possible errors that can be returned from
	/// `close`, I do not believe this is can realistically happen, and even if there is some error,
	/// the caller is unlikely to be able to do something about it. Hence, this method does not
	/// return a `Result`.
	fn close(self);
}
