//! This module defines a number of standard allocators. Use whichever is most appropriate

mod watermark;

pub use watermark::WaterMark;

use core::mem;
use core::ptr::NonNull;

/// A helper function to track allocations in case things are going very wrong.
#[cfg(feature = "log-allocations")]
fn track_allocation(pointer: NonNull<u8>, size: usize) {
	let end = pointer.as_ptr().wrapping_add(size);
	const LEN: u8 = 2 * mem::size_of::<usize>() as u8;
	let mut buf = [0; LEN as usize];
	let p = crate::util::usize_to_string(&mut buf, pointer.as_ptr() as usize, 16, LEN).unwrap();
	let mut buf = [0; LEN as usize];
	let e = crate::util::usize_to_string(&mut buf, end as usize, 16, LEN).unwrap();
	let mut buf = [0; LEN as usize];
	let sx = crate::util::usize_to_string(&mut buf, size, 16, 1).unwrap();
	let mut buf = [0; LEN as usize];
	let sd = crate::util::usize_to_string(&mut buf, size, 10, 1).unwrap();
	crate::log::debug(&["  alloc ", p, " - ", e, " (0x", sx, " - ", sd, ")"]);
}

/// A helper function to track allocations in case things are going very wrong.
#[cfg(not(feature = "log-allocations"))]
#[inline(always)]
fn track_allocation(pointer: NonNull<u8>, size: usize) {
	let _ = (pointer, size);
}

/// A helper function to track deallocations in case things are going very wrong.
#[cfg(feature = "log-allocations")]
fn track_deallocation(pointer: NonNull<u8>, size: usize) {
	let end = pointer.as_ptr().wrapping_add(size);
	const LEN: u8 = 2 * mem::size_of::<usize>() as u8;
	let mut buf = [0; LEN as usize];
	let p = crate::util::usize_to_string(&mut buf, pointer.as_ptr() as usize, 16, LEN).unwrap();
	let mut buf = [0; LEN as usize];
	let e = crate::util::usize_to_string(&mut buf, end as usize, 16, LEN).unwrap();
	let mut buf = [0; LEN as usize];
	let sx = crate::util::usize_to_string(&mut buf, size, 16, 1).unwrap();
	let mut buf = [0; LEN as usize];
	let sd = crate::util::usize_to_string(&mut buf, size, 10, 1).unwrap();
	crate::log::debug(&["dealloc ", p, " - ", e, " (0x", sx, " - ", sd, ")"]);
}

/// A helper function to track deallocations in case things are going very wrong.
#[cfg(not(feature = "log-allocations"))]
#[inline(always)]
fn track_deallocation(pointer: NonNull<u8>, size: usize) {
	let _ = (pointer, size);
}
