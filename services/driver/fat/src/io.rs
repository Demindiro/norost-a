//! # Support for `rust-fatfs` I/O traits.

use crate::*;
use core::ptr::NonNull;
use fatfs::*;

pub static mut ADDRESS: usize = 0;
pub static mut UUID: kernel::ipc::UUID = kernel::ipc::UUID::new(0);

pub struct GlobalIO<'a> {
	buffer: &'a mut kernel::Page,
	position: u64,
	buffer_sector: u64,
	max_position: u64,
	dirty: bool,
	need_fetch: bool,
}

impl<'a> GlobalIO<'a> {
	pub fn new(buffer: &'a mut kernel::Page) -> Self {
		let mut slf = Self {
			buffer,
			position: 0,
			dirty: false,
			need_fetch: true,
			max_position: 512 * 32, // TODO
			buffer_sector: u64::MAX,
		};
		slf.fetch();
		slf
	}

	fn seek_sector(&self) -> u64 {
		self.position / kernel::Page::SIZE as u64
	}

	fn buffer_sector(&self) -> u64 {
		self.buffer_sector
	}

	fn seek_offset(&self) -> usize {
		self.position as usize & kernel::Page::MASK
	}

	fn fetch(&mut self) {
		self.buffer_sector = self.seek_sector();

		unsafe {
			*dux::ipc::transmit() = kernel::ipc::Packet {
				opcode: Some(kernel::ipc::Op::Read.into()),
				address: ADDRESS,
				uuid: UUID,
				data: Some(core::ptr::NonNull::from(&*self.buffer)),
				length: kernel::Page::SIZE,
				offset: self.buffer_sector,
				flags: 0,
				id: 0,
				name: None,
				name_len: 0,
			};
		}
		loop {
			let pkt = dux::ipc::receive();
			if pkt.address != unsafe { ADDRESS } {
				panic!();
				pkt.defer();
				//unsafe { kernel::io_wait(u64::MAX) };
				unsafe { kernel::io_wait(10_000_000) };
				continue;
			}
			break;
		}
	}

	fn max_seek(&self) -> u64 {
		self.max_position
	}
}

impl IoBase for GlobalIO<'_> {
	type Error = ();
}

impl Read for GlobalIO<'_> {
	fn read(&mut self, data: &mut [u8]) -> Result<usize, Self::Error> {
		let mut i = 0;
		while i < data.len() {
			if self.position >= self.max_seek() {
				break;
			}
			if self.seek_sector() != self.buffer_sector() {
				self.flush()?;
				self.fetch();
				assert_eq!(self.seek_sector(), self.buffer_sector());
			}
			data[i] = unsafe { self.buffer.as_ref()[self.seek_offset()] };
			self.position += 1;
			i += 1;
		}
		Ok(i)
	}
}

impl Write for GlobalIO<'_> {
	fn write(&mut self, data: &[u8]) -> Result<usize, Self::Error> {
		let mut i = 0;
		while i < data.len() {
			if self.position >= self.max_seek() {
				break;
			}
			if self.seek_sector() != self.buffer_sector() {
				self.flush()?;
				if data.len() <= self.buffer.as_ref().len() {
					self.fetch();
					assert_eq!(self.seek_sector(), self.buffer_sector());
				}
			}
			let so = self.seek_offset();
			self.buffer.as_mut()[so] = data[i];
			self.position += 1;
			i += 1;
			self.dirty = true;
		}
		Ok(i)
	}

	fn flush(&mut self) -> Result<(), Self::Error> {
		if !self.dirty {
			return Ok(());
		}

		self.buffer_sector = self.seek_sector();

		unsafe {
			*dux::ipc::transmit() = kernel::ipc::Packet {
				opcode: Some(kernel::ipc::Op::Write.into()),
				address: ADDRESS,
				uuid: UUID,
				data: Some(core::ptr::NonNull::from(&*self.buffer)),
				length: kernel::Page::SIZE,
				offset: self.buffer_sector,
				flags: 0,
				id: 0,
				name: None,
				name_len: 0,
			};
		}
		loop {
			let pkt = dux::ipc::receive();
			if pkt.address != unsafe { ADDRESS } {
				pkt.defer();
				//unsafe { kernel::io_wait(u64::MAX) };
				unsafe { kernel::io_wait(1_000_000) };
				continue;
			}
			break;
		}
		self.dirty = false;
		Ok(())
	}
}

impl Seek for GlobalIO<'_> {
	fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
		self.position = match pos {
			SeekFrom::Start(p) => p,
			SeekFrom::Current(p) => {
				if p > 0 {
					self.position.checked_add(p as u64).unwrap_or(u64::MAX)
				} else {
					self.position.checked_sub((-p) as u64).ok_or(())?
				}
			}
			SeekFrom::End(p) => self.max_seek().checked_sub((-p) as u64).ok_or(())?,
		};
		Ok(self.position)
	}
}

impl Drop for GlobalIO<'_> {
	fn drop(&mut self) {
		// Panicking is tempting, but also a bad idea in a Drop handler
		match self.flush() {
			Ok(()) => (),
			Err(()) => kernel::sys_log!("failed to flush device on drop"),
		}
	}
}
