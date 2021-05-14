//! Driver for CPIO archives.
//!
//! It's technically not a filesystem, but it is normally used in conjunction with a filesystem,
//! hence it's here.
//!
//! This driver is a dependency of initramfs.
//!
//! ## Format
//!
//! Copied from kernel.org:
//!
//! ```
//! *       is used to indicate "0 or more occurrences of"
//! (|)     indicates alternatives
//! +       indicates concatenation
//! GZIP()  indicates the gzip(1) of the operand
//! ALGN(n) means padding with null bytes to an n-byte boundary
//!
//! initramfs  := ("\0" | cpio_archive | cpio_gzip_archive)*
//!
//! cpio_gzip_archive := GZIP(cpio_archive)
//!
//! cpio_archive := cpio_file* + (<nothing> | cpio_trailer)
//!
//! cpio_file := ALGN(4) + cpio_header + filename + "\0" + ALGN(4) + data
//!
//! cpio_trailer := ALGN(4) + cpio_header + "TRAILER!!!\0" + ALGN(4)
//! ```
//!
//! ## References
//!
//! [`initramfs` buffer format][spec]
//!
//! [spec]: https://www.kernel.org/doc/html/latest/driver-api/early-userspace/buffer-format.html

use crate::alloc::{ReserveError, Vec};
use core::alloc::{AllocError, Allocator};
use core::mem;

/// Structure representing a CPIO archive
pub struct Archive<'a, A>
where
	A: Allocator,
{
	/// A list of files present in the archive
	pub files: Vec<File<'a>, A>,
}

/// Structure representing a CPIO file
pub struct File<'a> {
	/// File inode number
	pub inode: u32,
	/// File permissions
	pub permissions: super::Permissions,
	/// File user ID (UID)
	pub user_id: u32,
	/// File group ID (GID)
	pub group_id: u32,
	/// Number of links
	pub link_count: u32,
	/// Last modification time
	pub modification_time: u32,
	/// File data
	pub data: &'a [u8],
	/// The name of the file.
	pub name: &'a str,
}

/// Structure representing a CPIO file header
///
/// Only version "070701" and "070702" are supported. There is apparently a "070707" but
/// I don't know how common it is.
///
/// [CPIO "070707"](https://www.mkssoftware.com/docs/man4/cpio.4.asp)
/// [Other CPIO "070707"](https://www.systutorials.com/docs/linux/man/5-cpio/)
#[repr(C)]
struct FileHeader {
	/// Magic value that must be equivalent to either the string "070701" or "070702"
	magic: [u8; 6],
	/// File inode number
	inode: [u8; 8],
	/// File mode and permissions
	mode: [u8; 8],
	/// File user ID (UID)
	user_id: [u8; 8],
	/// File group ID (GID)
	group_id: [u8; 8],
	/// Number of links
	link_count: [u8; 8],
	/// Last modification time
	modification_time: [u8; 8],
	/// Size of file.
	file_size: [u8; 8],
	/// Major part of file device number
	device_major: [u8; 8],
	/// Minor part of file device number
	device_minor: [u8; 8],
	/// Major part of device node reference
	device_major_reference: [u8; 8],
	/// Minor part of device node reference
	device_minor_reference: [u8; 8],
	/// Length of the filename, including final `\0` terminator
	name_size: [u8; 8],
	/// Checksum if `magic == "070702"`. Otherwise zero.
	/// The checksum is a simple 32 bit unsigned sum over each of the bytes in the file data.
	/// It's very weak and silly but it is what it is.
	checksum: [u8; 8],
}

/// Enum of possible errors that can occur while parsing an archive.
pub enum ArchiveError {
	/// There is something wrong with a file. The `u32` indicates the file's index.
	FileError(FileError, u32),
	/// An error occured wrt. heap allocation.
	ReserveError(ReserveError),
}

/// Enum of possible errors that can occur while parsing a file.
pub enum FileError {
	InvalidMagic,
	InvalidChecksum,
	Truncated,
	InvalidHexaDecimal,
	InvalidFileName,
}

impl<'a, A> Archive<'a, A>
where
	A: Allocator,
{
	/// Parses the CPIO archive data at the given address. If an archive trailer was
	/// encountered, it'll return early and return a slice to the remainder of the
	/// data.
	pub fn parse(mut data: &'a [u8], allocator: A) -> Result<(Self, &'a [u8]), ArchiveError> {
		let mut files = Vec::new_in(allocator);
		let mut offset = 0;
		loop {
			if data.len() < offset {
				return Err(ArchiveError::FileError(
					FileError::Truncated,
					files.len() as u32,
				));
			}
			match File::parse(&data[offset..]) {
				Ok((file, offt)) => {
					if (file.name == "TRAILER!!!" && file.data.len() == 0) {
						return Ok((Self { files }, &[]));
					}
					offset = (offset + offt + 3) & !3;
					files.try_push(file)?;
				}
				Err(e) => return Err(ArchiveError::FileError(e, files.len() as u32)),
			}
		}
	}
}

impl<'a> File<'a> {
	/// Parses the CPIO  data at the given address.
	fn parse(data: &'a [u8]) -> Result<(Self, usize), FileError> {
		if mem::size_of::<FileHeader>() < data.len() {
			// SAFETY: the header is large enough
			let header = unsafe { &*(data as *const _ as *const FileHeader) };

			if !(&header.magic[..5] == b"07070" && b"12".contains(&header.magic[5])) {
				for c in header.magic.iter() {}
				return Err(FileError::InvalidMagic);
			}

			fn parse_hex(hex: &[u8; 8]) -> Result<u32, FileError> {
				let mut n = 0u32;
				for &c in hex {
					n *= 16;
					n += match c {
						b'0'..=b'9' => c - b'0',
						b'a'..=b'a' => c + 10 - b'a',
						b'A'..=b'F' => c + 10 - b'A',
						_ => return Err(FileError::InvalidHexaDecimal),
					} as u32;
				}
				Ok(n)
			}

			let inode = parse_hex(&header.inode)?;
			let mode = parse_hex(&header.mode)?;
			let user_id = parse_hex(&header.user_id)?;
			let group_id = parse_hex(&header.group_id)?;
			let link_count = parse_hex(&header.link_count)?;
			let modification_time = parse_hex(&header.modification_time)?;
			let file_size = parse_hex(&header.file_size)?;
			// TODO what am I supposed to do with these?
			//let device_major = parse_hex(&header.device_major);
			//let device_minor = parse_hex(&header.device_minor);
			//let device_major_reference = parse_hex(&header.device_major_reference);
			//let device_minor_reference = parse_hex(&header.device_minor_reference);
			let name_size = parse_hex(&header.name_size)?;
			// TODO should we just skip the checksum? It's pretty much useless anyways.
			//let checksum = parse_hex(&header.checksum);

			// This won't panic as we verified the length of the header before
			let dt = &data[mem::size_of::<FileHeader>()..];

			let name_size = name_size as usize;
			if dt.len() < name_size {
				return Err(FileError::Truncated);
			}
			let name = core::str::from_utf8(&dt[..name_size - 1])
				.map_err(|_| FileError::InvalidFileName)?;
			if data.len() < name_size {
				return Err(FileError::Truncated);
			}
			let offset = (mem::size_of::<FileHeader>() + name_size + 3) & !3;
			let dt = &data[offset..];

			let file_size = file_size as usize;
			if dt.len() < file_size {
				return Err(FileError::Truncated);
			}
			Ok((
				Self {
					inode,
					permissions: (mode as u16).into(),
					user_id,
					group_id,
					link_count,
					modification_time,
					name,
					data: &dt[..file_size],
				},
				offset + file_size,
			))
		} else {
			Err(FileError::Truncated)
		}
	}
}

impl From<ReserveError> for ArchiveError {
	fn from(error: ReserveError) -> Self {
		Self::ReserveError(error)
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::alloc::allocators::WaterMark;
	use crate::log;

	const HEAP_ADDRESS: *mut u8 = 0x8100_0000 as *mut _;
	const ARCHIVE: &[u8] = include_bytes!("../../../initfs.cpio");

	#[test_case]
	test!(listdir() {
		let heap = unsafe { WaterMark::new(core::ptr::NonNull::new(HEAP_ADDRESS).unwrap(), 4096) };
		let archive = Archive::parse(ARCHIVE, heap).map_err(|_| ()).unwrap();
		for file in archive.0.files.iter() {
			log::debug_str(file.name);
		}
	});
}
