//! # Arena allocator
//!
//! A arena allocator functions allocates objects of a specific size & alignment.
//!
//! It keeps track of free slots with a linked lists of pointers, terminated by a null pointer.
//! Consequently, the minimum alignment is that of an `usize`.
//!
//! The minimum size of a slot is 2 times the size of an `usize` as each slot is an enum. This
//! is so indexing in the arena is possible.
//!
//! If no free slots are available, a new page is allocated & the linked list is repopulated.
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

/// The value inside a slot.
union SlotValue<T> {
	item: mem::ManuallyDrop<T>,
	next: usize,
}

/// A single slot
struct Slot<T> {
	/// The amount of references to this slot. Is usize::MAX if free.
	ref_counter: AtomicUsize,
	/// The value inside a slot.
	value: SlotValue<T>,
}

/// An arena allocator
pub struct Arena<T> {
	/// The start pointer to all free slots.
	slots: NonNull<Slot<T>>,
	/// An index to the next free slot.
	next: AtomicUsize,
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

#[derive(Debug)]
pub enum RemoveError {
	/// There is no item.
	NoItem,
	/// Something else is referencing the item.
	Referenced,
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
		assert!(crate::arch::Page::SIZE >= mem::size_of::<Slot<T>>());
		Self {
			slots: address.cast(),
			next: AtomicUsize::new(usize::MAX),
			capacity: AtomicUsize::new(0),
			max: bytes / mem::size_of::<Slot<T>>(),
		}
	}

	/// Allocate a slot and return the index.
	pub fn insert<'a>(&'a self, item: T) -> Result<usize, InsertError> {
		loop {
			let index = self.next.load(Ordering::Relaxed);
			if index != usize::MAX {
				let slot = unsafe { &*self.slots.as_ptr().add(index) };
				if slot
					.ref_counter
					.compare_exchange_weak(usize::MAX, 0, Ordering::Relaxed, Ordering::Relaxed)
					.is_ok()
				{
					// SAFETY: we have exclusive access to the slot.
					unsafe {
						let slot = &mut *self.slots.as_ptr().add(index);
						// Even if another thread would read this value again before we finish
						// writing, it'll fail the compare_exchange check anyways.
						self.next.store(slot.value.next, Ordering::Relaxed);
						slot.value.item = mem::ManuallyDrop::new(item);
					}
					return Ok(index);
				}
			} else {
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
						Page::new(self.slots).unwrap(),
						Map::Private(page),
						vms::RWX::RW,
						vms::Accessibility::KernelGlobal,
					)
					.expect("Page was already mapped");
					// FIXME *sigh*
					unsafe {
						self.slots.as_ptr().write(Slot {
							ref_counter: AtomicUsize::new(0),
							value: SlotValue {
								item: mem::ManuallyDrop::new(item),
							},
						})
					};
					self.capacity.store(cap + 1, Ordering::Relaxed);
					return Ok(0); // FIXME WTF
				}
			}
		}
	}

	/// Attempt to free a slot. Returns the original item if successful.
	pub fn remove(&self, index: usize) -> Result<T, RemoveError> {
		(index < self.capacity.load(Ordering::Relaxed))
			.then(|| ())
			.ok_or(RemoveError::NoItem)?;
		loop {
			let ptr = unsafe { self.slots.as_ptr().add(index) };
			let slot = unsafe { &*ptr };
			let val = slot.ref_counter.load(Ordering::Relaxed);
			if val > 0 {
				return Err(RemoveError::Referenced);
			} else if val == usize::MAX {
				return Err(RemoveError::NoItem);
			} else if slot
				.ref_counter
				.compare_exchange_weak(val, usize::MAX, Ordering::Relaxed, Ordering::Relaxed)
				.is_ok()
			{
				// SAFETY: we have exclusive access at this point.
				let item = unsafe { ptr::read(ptr).value.item };
				return Ok(mem::ManuallyDrop::into_inner(item));
			}
		}
	}
}

impl<T> Arena<T>
where
	T: Sync,
{
	/// Return an item at an index, if any.
	pub fn get<'a>(&'a self, index: usize) -> Option<Guard<'a, T>> {
		(index < self.capacity.load(Ordering::Relaxed))
			.then(|| unsafe { &*self.slots.as_ptr().add(index) })
			.and_then(|slot| loop {
				let val = slot.ref_counter.load(Ordering::Relaxed);
				if val != usize::MAX {
					if slot
						.ref_counter
						.compare_exchange_weak(val, val + 1, Ordering::Relaxed, Ordering::Relaxed)
						.is_ok()
					{
						let (item, counter) = (unsafe { &*slot.value.item }, &slot.ref_counter);
						return Some(Guard { item, counter });
					}
				} else {
					return None;
				}
			})
	}

	/// Iterate over all the elements in the arena.
	pub fn iter<'a>(&'a self) -> impl Iterator<Item = Guard<'a, T>> + 'a {
		let cap = self.capacity.load(Ordering::Relaxed);
		(0..cap).flat_map(move |i| self.get(i))
	}
}

// SAFETY: The allocator is explicitly designed to be thread safe.
unsafe impl<T> Sync for Arena<T> {}

/// A structure used to safely & automatically update the reference counters.
pub struct Guard<'a, T> {
	item: &'a T,
	counter: &'a AtomicUsize,
}

impl<T> Drop for Guard<'_, T> {
	fn drop(&mut self) {
		self.counter.fetch_sub(1, Ordering::Relaxed);
	}
}

impl<T> Deref for Guard<'_, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.item
	}
}
