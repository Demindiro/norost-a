//! Driver for Linux's `initramfs` format.
//!
//! `initramfs` was chosen for early userspace loading since it's simple and already widely
//! supported.
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

use super::cpio;
use crate::alloc::{Box, Vec};
use crate::log;
use core::alloc::Allocator;
use core::mem;
use core::pin::Pin;

/// Structure representing an initramfs block
// TODO M should be a special sort of "page allocator". It must NOT implement core::alloc::Allocator,
// as it'll need some special properties for TLB.
struct InitRAMFS<A>
where
	A: Allocator,
{
	/// The root (`/`) of the filesystem.
	root: Vec<Node<A>, A>,
}

/// A node in the filesystem tree.
///
/// There are no UID, GID or modification date fields as those are not necessary during early
/// boot.
struct Node<A>
where
	A: Allocator,
{
	/// The name of the node.
	name: Box<str, A>,
	/// The permissions on this node
	permissions: super::Permissions,
	/// Node data that is file-type specific
	data: Branch<A>,
}

/// File-type specific data.
enum Branch<A>
where
	A: Allocator,
{
	/// A regular file
	File(File),
	/// Another branch, i.e. a directory.
	Directory(Vec<Node<A>, A>),
}

/// Structure representing a regular file
struct File {
	/// The data of the file.
	data: (), /* TODO structure describing a range of memory pages */
}

/// Enum representing possible errors when trying to unpack a CPIO archive.
pub enum Error {
	Archive(cpio::ArchiveError),
	AllocError,
	/// A regular file is a path component of another file, e.g.
	///
	/// ```
	/// /foo.txt          <-- file
	/// /foo.txt/bar.txt  <-- file inside file, which isn't possible
	/// ```
	FileTreatedAsDirectory,
}

impl<A> InitRAMFS<A>
where
	A: Allocator + Copy,
{
	/// Attempts to create a filesystem from the given CPIO data.
	pub fn parse(cpio_data: &[u8], allocator: A) -> Result<Self, Error> {
		let cpio = cpio::Archive::parse(cpio_data, allocator).map_err(Error::Archive)?;
		let mut root = Vec::new_in(allocator);
		for file in cpio.0.files.iter() {
			let mut iter = file.name.split("/").peekable();
			let mut vec: &mut Vec<Node<A>, A> = &mut root;
			loop {
				let comp = iter.next().unwrap();
				if iter.peek().is_some() {
					// find() causes some complaints about mutable borrows, so use position and index
					// (which should optimize out any bounds checks anyways)
					vec = if let Some(i) = vec.iter_mut().position(|n| n.name.as_ref() == comp) {
						vec[i]
							.data
							.as_directory_mut()
							.ok_or(Error::FileTreatedAsDirectory)?
					} else {
						vec.try_push(Node {
							name: Box::try_from_str(comp, allocator)
								.ok()
								.ok_or(Error::AllocError)?,
							permissions: file.permissions,
							data: Branch::Directory(Vec::new_in(allocator)),
						})
						.map(|d| d.data.as_directory_mut().unwrap())
						.ok()
						.ok_or(Error::AllocError)?
					};
				} else {
					vec.try_push(Node {
						name: Box::try_from_str(comp, allocator)
							.ok()
							.ok_or(Error::AllocError)?,
						permissions: file.permissions,
						data: Branch::File(File { data: () }),
					});
					break;
				}
			}
		}
		Ok(Self { root })
	}
}

impl<A> Branch<A>
where
	A: Allocator,
{
	fn as_directory(&self) -> Option<&Vec<Node<A>, A>> {
		if let Self::Directory(d) = self {
			Some(d)
		} else {
			None
		}
	}

	fn as_directory_mut(&mut self) -> Option<&mut Vec<Node<A>, A>> {
		if let Self::Directory(d) = self {
			Some(d)
		} else {
			None
		}
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
	test!(list_tree() {
		let heap = unsafe { WaterMark::new(core::ptr::NonNull::new(HEAP_ADDRESS).unwrap(), 0x10_000) };
		let ramfs = InitRAMFS::parse(ARCHIVE, &heap).map_err(|_| ()).unwrap();
		fn print<'a>(node: &'a Node<&'a WaterMark>, buf: &mut [&'a str; 64], level: usize) {
			buf[level] = node.name.as_ref();
			buf[level + 1] = if node.data.as_directory().is_some() { "/" } else { "" };
			log::debug(&buf[..level + 2]);
			if let Some(v) = node.data.as_directory() {
				buf[level] = "  ";
				for node in v.iter() {
					print(node, buf, level + 1);
				}
			}
		}
		let mut buf = ["  "; 64];
		for node in ramfs.root.iter() {
			print(node, &mut buf, 0);
		}
	});
}
