//! Virtual file system abstraction
//!
//! This is an universal interface that abstracts other filesystems. It also caches
//! data it reads to speed up accesses.
//!
//! The VFS is inside the kernel mainly because it's needed to mount the early userspace
//! filesystem. While some trickery may be possible to move it to an external service, ding so is
//! likely more efforts than it's worth.
//!
//! Having the VFS inside the kernel should also speed up access to cached pages, which is always
//! a Good Thing(tm).
//!
//! ## TODO
//!
//! Figure out how to integrate this with the **userspace** memory manager service.
//!
//! One way to achieve it would be to have a small internal buffer (could even be a section instead
//! of using the heap) and extending it with memory from the memory manager services as needed.

use crate::alloc::{Box, Vec};
use crate::driver::fs;
use core::alloc::Allocator;

/// A structure containing the VFS state.
pub struct VirtualFileSystem<A>
where
	A: Allocator,
{
	/// A list of all memory ranges in use by the VFS.
	memories: Vec<Memory, A>,
	/// A list of all mounted filesystems. The key is simply a string describing the path. The
	/// value is a pointer to an interface of the filesystem.
	mount_points: Vec<(Box<str, A>, Box<dyn fs::FileSystem, A>), A>,
}

/// A structure describing a range of memory
struct Memory {
	/// The start address of this memory
	start: *const u8,
	/// The end address of this memory
	end: *const u8,
}

impl<A> VirtualFileSystem<A> where A: Allocator {}
