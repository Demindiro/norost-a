//! # Arena allocator
//!
//! A arena allocator functions allocates objects of a specific size & alignment.
//!
//! It does not keep track of free slots.
//!
//! If no free slots are available, a new page is allocated.
//!
//! It is currently unable to free pages, so memory peaks have permanent effects.

use crate::arch::vms::VirtualMemorySystem;
use crate::arch::*;
use crate::memory;
use core::mem;
use core::ops::Deref;
use core::ptr;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};

/// An arena allocator
pub struct Arena<T> {
	/// The start pointer to the list.
	slots: NonNull<T>,
	/// The amount of allocated slots.
	capacity: AtomicUsize,
	/// The maximum amount of slots.
	max: usize,
}

#[derive(Debug)]
pub enum InsertError {
	/// There are no free slots.
	NoFreeSlots,
	/// There is no free memory.
	NoMemory,
}

impl<T> Arena<T> {
	/// Create a new `Arena` with governance over the given range of memory.
	///
	/// # Safety
	///
	/// The memory range may not be in use by anything else.
	///
	/// The `address` must be properly aligned.
	pub const unsafe fn new(address: NonNull<T>, bytes: usize) -> Self {
		// Ensure we can fit T in a single page. This keeps things simple for now.
		// TODO use `const _: usize = ...` for this somehow.
		assert!(crate::arch::Page::SIZE >= mem::size_of::<T>());
		Self {
			slots: address.cast(),
			capacity: AtomicUsize::new(0),
			max: bytes / mem::size_of::<T>(),
		}
	}

	/// Allocate a new slot and return the index.
	pub fn insert_with<'a>(&'a self, f: impl FnOnce(usize) -> T) -> Result<usize, InsertError> {
		loop {
			let cap = self.capacity.load(Ordering::Relaxed);
			if cap == usize::MAX {
				// Something else is already loading a page, just wait & retry.
			} else if cap == self.max {
				return Err(InsertError::NoFreeSlots);
			} else if self
				.capacity
				.compare_exchange_weak(cap, usize::MAX, Ordering::Relaxed, Ordering::Relaxed)
				.is_ok()
			{
				let page = memory::allocate().map_err(|_| InsertError::NoMemory)?;
				// FIXME don't allocate just the first page ya fuckwit
				VMS::add(
					Page::new(self.slots.cast()).unwrap(),
					Map::Private(page),
					vms::RWX::RW,
					vms::Accessibility::KernelGlobal,
				)
				.expect("Page was already mapped");
				// FIXME *sigh*
				unsafe { self.slots.as_ptr().write(f(cap)) };
				self.capacity.store(cap + 1, Ordering::Relaxed);
				return Ok(cap);
			}
		}
	}
}

impl<T> Arena<T>
where
	T: Sync,
{
	/// Return an item at an index, if any.
	pub fn get<'a>(&'a self, index: usize) -> Option<&'a T> {
		(index < self.capacity.load(Ordering::Relaxed))
			.then(|| unsafe { &*self.slots.as_ptr().add(index) })
	}
}

// SAFETY: The allocator is explicitly designed to be thread safe.
unsafe impl<T> Sync for Arena<T> {}
