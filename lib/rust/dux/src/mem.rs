//! # Address range reservations.
//!
//! As the kernel does not have any form of CoW, it is impractical to allocate a large range
//! of pages upfront. However, simply leaving the gaps open and allocating anywhere may
//! interfere with some systems such as the memory allocator.
//!
//! This module provides utilities for keeping track of reserved ranges.

use crate::util;
use crate::{Page, RWX};
use core::cell::Cell;
use core::mem;
use core::ops;
use core::ptr;
use core::ptr::NonNull;
use core::slice;
use core::sync::atomic::{AtomicBool, AtomicU16, AtomicUsize, Ordering};

/// A single memory range.
#[derive(Clone, Copy)]
pub struct Range {
	start: Option<Page>,
	/// end is *inclusive*, i.e. the offset bits are all `1`s.
	end: *mut kernel::Page,
}

pub struct Reservations<L>
where
	L: AsMut<[Range]>,
{
	count: usize,
	list: L,
}

impl<L> Reservations<L>
where
	L: AsMut<[Range]>,
{
	/// Insert a memory reservation entry. The index must be lower than reserved_count.
	fn insert(
		&mut self,
		index: usize,
		start: Page,
		end: NonNull<kernel::Page>,
	) -> Result<(), ReserveError> {
		let list = self.list.as_mut();
		(self.count < list.len())
			.then(|| ())
			.ok_or(ReserveError::NoMemory)?;
		self.count += 1;
		// Shift all entries at and after the index up.
		for i in (index + 1..self.count).rev() {
			list[i] = list[i - 1];
		}
		// Write the entry.
		list[index] = Range {
			start: Some(start),
			end: end.as_ptr(),
		};
		Ok(())
	}

	pub fn reserve_range(
		&mut self,
		address: Option<Page>,
		count: usize,
	) -> Result<Page, ReserveError> {
		if let Some(_address) = address {
			// Do a binary search, check if there is enough space & insert if so.
			todo!()
		} else {
			// Find the first range with enough space.
			// TODO maybe it's better if we try to find the tightest space possible? Or maybe
			// the widest space instead?
			let mut prev_end = Page::NULL_PAGE_END.cast::<u8>();
			for i in 0..self.count {
				let mm = &self.list.as_mut()[i];
				let start = prev_end.wrapping_add(1);
				let end = start.wrapping_add(count * Page::SIZE - 1);
				if prev_end < start
					&& end
						< mm.start
							.map(|p| p.as_ptr().cast())
							.unwrap_or_else(ptr::null_mut)
				{
					// There is enough space, so use it.
					let start = unsafe { Page::new_unchecked(start.cast()) };
					let end = NonNull::new(end).unwrap().cast();
					return self.insert(i, start, end).map(|_: ()| start);
				}
				prev_end = mm.end.cast();
			}
			Err(ReserveError::NoSpace)
		}
	}

	pub fn unreserve_range(&mut self, address: Page, _count: usize) -> Result<(), UnreserveError> {
		let list = self.list.as_mut();
		let i = list[..self.count]
			.binary_search_by(|e| {
				e.start
					.map(|p| p.as_ptr())
					.unwrap_or_else(ptr::null_mut)
					.cmp(&address.as_ptr())
			})
			// TODO check for size
			.map_err(|_| UnreserveError::InvalidAddress)?;
		self.count -= 1;
		for i in i..self.count {
			list[i] = list[i + 1];
		}
		Ok(())
	}

	/// Allocate a range of pages.
	///
	/// This automatically reserves a range.
	pub fn allocate_range(
		&mut self,
		address: Option<Page>,
		count: usize,
		flags: RWX,
	) -> Result<Page, ReserveError> {
		let address = self.reserve_range(address, count)?;
		let ret = unsafe { kernel::mem_alloc(address.as_ptr(), count, flags.into()) };
		match ret.status {
			kernel::Return::OK => Ok(address),
			r => unreachable!("{}", r),
		}
	}

	/// Deallocate a range of pages.
	///
	/// This automatically unreserves a range.
	///
	/// # Safety
	///
	/// The pages are no longer in use.
	///
	/// # Panics
	///
	/// The pages are not reserved or allocated.
	pub unsafe fn deallocate_range(&mut self, address: Page, count: usize) {
		self.unreserve_range(address, count)
			.expect("failed to deallocate range");
		let ret = kernel::mem_dealloc(address.as_ptr(), count);
		match ret.status {
			kernel::Return::OK => (),
			kernel::Return::MEMORY_NOT_ALLOCATED => panic!("pages were not allocated"),
			r => unreachable!("{}", r),
		}
	}
}

#[derive(Debug)]
pub enum ReserveError {
	/// Failed to allocate memory
	NoMemory,
	/// There is no free range large enough
	NoSpace,
}

#[derive(Debug)]
pub enum UnreserveError {
	/// There is no entry with the given address.
	InvalidAddress,
	/// The size of the entry is too large.
	SizeTooLarge,
}

/// Functions & structures intended for `crate::ipc` but defined here because it depends strongly
/// on `GLOBAL`.
pub(crate) mod ipc {

	use super::*;
}
