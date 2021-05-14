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
use crate::arch::Page;
use crate::{arch, log, MEMORY_MANAGER};
use core::alloc::Allocator;
use core::ptr::NonNull;
use core::{mem, ptr};

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
	File(File<A>),
	/// Another branch, i.e. a directory.
	Directory(Vec<Node<A>, A>),
}

/// Structure representing a regular file
struct File<A>
where
	A: Allocator,
{
	/// The total size of the file in bytes.
	size: usize,
	/// The data of the file, which is stored in a number of memory pages.
	pages: Box<[NonNull<Page>], A>,
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
	MemoryManagerError(crate::memory::AllocateError),
}

impl<A> InitRAMFS<A>
where
	A: Allocator + Copy,
{
	/// Attempts to create a filesystem from the given CPIO data.
	// FIXME this leaks memory pages if an error occurs (which should generally not be an issue
	// considering the boot process can't recover from this but still).
	pub fn parse(cpio_data: &[u8], allocator: A) -> Result<Self, Error> {
		let cpio = cpio::Archive::parse(cpio_data, allocator).map_err(Error::Archive)?;
		let mut root = Vec::new_in(allocator);
		for file in cpio.0.files.iter() {
			let mut iter = file.name.split("/").peekable();
			let mut vec: &mut Vec<Node<A>, A> = &mut root;
			loop {
				let comp = iter.next().unwrap();
				if iter.peek().is_some() {
					// Insert or get a directory node.
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
								.map_err(|_| Error::AllocError)?,
							permissions: file.permissions,
							data: Branch::Directory(Vec::new_in(allocator)),
						})
						.map(|d| d.data.as_directory_mut().unwrap())
						.map_err(|_| Error::AllocError)?
					};
				} else {
					// Copy the data to memory pages.
					let pages = (file.data.len() + arch::PAGE_MASK) & !arch::PAGE_MASK;
					let pages = pages / arch::PAGE_SIZE;
					let mut pages = Box::try_new_uninit_slice_in(pages, allocator)
						.map_err(|_| Error::AllocError)?;
					let mut data = file.data;
					if pages.len() > 0 {
						for i in 0..pages.len() - 1 {
							let page = MEMORY_MANAGER
								.lock()
								.allocate(0)
								.map_err(Error::MemoryManagerError)?;
							// SAFETY: the page we received points to valid memory and data is at
							// least one page size large.
							debug_assert!(data.len() >= arch::PAGE_SIZE);
							unsafe {
								ptr::copy_nonoverlapping(
									data.as_ptr(),
									page.cast().as_ptr(),
									arch::PAGE_SIZE,
								);
							}
							data = &data[arch::PAGE_SIZE..];
							pages[i].write(page);
						}
						let page = MEMORY_MANAGER
							.lock()
							.allocate(0)
							.map_err(Error::MemoryManagerError)?;
						// SAFETY: the page we received points to valid memory and data is at
						// least one page size large.
						debug_assert!(data.len() < arch::PAGE_SIZE);
						unsafe {
							ptr::copy_nonoverlapping(
								data.as_ptr(),
								page.cast().as_ptr(),
								data.len(),
							);
						}
						*pages.last_mut().unwrap().write(page);
					}
					// SAFETY: all pages are initialized.
					let pages = unsafe { pages.assume_init() };
					// Insert a file node.
					vec.try_push(Node {
						name: Box::try_from_str(comp, allocator).map_err(|_| Error::AllocError)?,
						permissions: file.permissions,
						data: Branch::File(File {
							size: file.data.len(),
							pages,
						}),
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

	fn as_file(&self) -> Option<&File<A>> {
		if let Self::File(f) = self {
			Some(f)
		} else {
			None
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::alloc::allocators::WaterMark;
	use crate::{log, util};

	const ARCHIVE: &[u8] = include_bytes!("../../../initfs.cpio");

	#[test_case]
	test!(list_tree() {
		let heap = MEMORY_MANAGER.lock().allocate(7).unwrap(); // Assume 512 page size is minimum.
		let heap = unsafe { WaterMark::new(heap.cast(), 0x10_000) };
		let ramfs = InitRAMFS::parse(ARCHIVE, &heap).map_err(|_| ()).unwrap();
		fn print<'a>(node: &'a Node<&'a WaterMark>, buf: &mut [&'a str; 64], level: usize) {
			buf[level] = node.name.as_ref();
			match &node.data {
				Branch::File(f) => {
					buf[level + 1] = "\t\t(size: ";
					// I can't be fucked dealing with this
					// Rust, please recognize that I am clearing the references afterwards. Thank you.
					let mut a = [0; 20];
					buf[level + 2] = unsafe { core::mem::transmute(util::usize_to_string(&mut a, f.size, 10, 1).unwrap()) };
					buf[level + 3] = ", pages: ";
					let mut a = [0; 20];
					buf[level + 4] = unsafe { core::mem::transmute(util::usize_to_string(&mut a, f.pages.len(), 10, 1).unwrap()) };
					buf[level + 5] = ")";
					log::debug(&buf[..level + 6]);
					// Poof! Gone! Rust pls.
					buf[level + 2] = "";
					buf[level + 4] = "";
				}
				Branch::Directory(d) => {
					buf[level + 1] = "/";
					log::debug(&buf[..level + 2]);
					buf[level] = "  ";
					for node in d.iter() {
						print(node, buf, level + 1);
					}
				}
			}
		}
		let mut buf = ["  "; 64];
		for node in ramfs.root.iter() {
			print(node, &mut buf, 0);
		}
	});
}
