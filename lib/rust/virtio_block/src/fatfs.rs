//! # Support for `rust-fatfs` I/O traits.
//!
//! Since there is no equivalent of `std::io` in `core`, `rust-fatfs` reimplemented these traits.
//! This module implements the traits for that crate.

use crate::*;
use ::fatfs::*;

/// A proxy between the device that keeps track of the read head & buffers the last accessed
/// sector.
pub struct Proxy<'a, 'b, A>
where
	A: Allocator + 'a
{
	device: &'b mut BlockDevice<'a, A>,
	position: u64,
	buffer: Sector,
	buffer_sector: u64,
	dirty: bool,
}

impl<'a, 'b, A> Proxy<'a, 'b, A>
where
	A: Allocator + 'a,
{
	pub fn new(device: &'b mut BlockDevice<'a, A>) -> Self {
		let mut slf = Self {
			device,
			position: 0,
			buffer: Sector([0; 512]),
			buffer_sector: 0,
			dirty: false,
		};
		slf.fetch();
		slf
	}

	fn seek_sector(&self) -> u64 {
		self.position >> 9
	}

	fn buffer_sector(&self) -> u64 {
		self.buffer_sector
	}

	fn seek_offset(&self) -> usize {
		(self.position & 0x1ff) as usize
	}

	fn fetch(&mut self) {
		self.buffer_sector = self.seek_sector();
		self.device.read(&mut self.buffer, self.buffer_sector);
	}

	fn max_seek(&self) -> u64 {
		self.device.capacity * self.buffer.len() as u64
	}
}

impl<'a, A> IoBase for Proxy<'a, '_, A>
where
	A: Allocator + 'a
{
	type Error = ();
}

impl<'a, A> Read for Proxy<'a, '_, A>
where
	A: Allocator + 'a
{
	fn read(&mut self, mut data: &mut [u8]) -> Result<usize, Self::Error> {
		let mut i = 0;
		while i < data.len() {
			if self.position >= self.max_seek() {
				break;
			}
			if self.seek_sector() != self.buffer_sector() {
				self.flush();
				self.fetch();
				assert_eq!(self.seek_sector(), self.buffer_sector());
			}
			data[i] = self.buffer[self.seek_offset()];
			self.position += 1;
			i += 1;
		}
		Ok(i)
	}
}

impl<'a, A> Write for Proxy<'a, '_, A>
where
	A: Allocator + 'a
{
	fn write(&mut self, mut data: &[u8]) -> Result<usize, Self::Error> {
		let mut i = 0;
		while i < data.len() {
			if self.position >= self.max_seek() {
				break;
			}
			if self.seek_sector() != self.buffer_sector() {
				self.flush();
				if data.len() <= self.buffer.len() {
					self.fetch();
					assert_eq!(self.seek_sector(), self.buffer_sector());
				}
			}
			let so = self.seek_offset();
			self.buffer[so] = data[i];
			self.position += 1;
			i += 1;
			self.dirty = true;
		}
		Ok(i)
	}

	fn flush(&mut self) -> Result<(), Self::Error> {
		if self.dirty {
			self.device.write(&mut self.buffer, self.buffer_sector);
			self.dirty = false;
		}
		Ok(())
	}
}

impl<'a, A> Seek for Proxy<'a, '_, A>
where
	A: Allocator + 'a
{
	fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
		self.position = match pos {
			SeekFrom::Start(p) => p,
			SeekFrom::Current(p) => if p > 0 {
				self.position.checked_add(p as u64).unwrap_or(u64::MAX)
			} else {
				self.position.checked_sub((-p) as u64).ok_or(())?
			}
			SeekFrom::End(p) => self.max_seek().checked_sub((-p) as u64).ok_or(())?,
		};
		Ok(self.position)
	}
}

impl<'a, A> Drop for Proxy<'a, '_, A>
where
	A: Allocator + 'a
{
	fn drop(&mut self) {
		self.flush();
	}
}
