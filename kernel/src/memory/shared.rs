//! Management of shared pages.

use super::{mem_allocate, mem_deallocate, AllocateError};
use crate::arch::{Page, PAGE_SIZE, PAGE_MASK};
use crate::sync::Mutex;
use core::mem;
use core::num::NonZeroU16;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicU16, Ordering};

/// Global list of reference counters.
static COUNTERS: Mutex<ReferenceCounters> = Mutex::new(ReferenceCounters {
	free: None,
	full: None,
});

/// Representation of a page that can be safely shared.
struct SharedPage {
	/// Pointer to the page.
	page: NonNull<Page>,
	/// Pointer to a counter. This counter is 16 bits by default as that is likely a reasonable
	/// number for "regular" systems. It may be desireable to be able to change this to 32/64...
	/// with a feature flag if you expect to share a mapping by a very high amount of tasks.
	counter: NonNull<AtomicU16>,
}

/// Structure returned if the reference count would overflow.
#[derive(Debug)]
struct ReferenceCountOverflow;

/// Structure used to manage reference counter tables.
///
/// It is a linked list of pages which have a header and a list of counters.
///
/// The header of each page has:
///
/// * A pointer to the next page (if any)
/// * A pointer to the previous page (if any)
/// * A `NonZeroU16` offset to the next free counter.
///
/// The "root" has two pointers: a pointer to a list of pages with free counters and a pointer
/// to a list of full tables. This keeps allocation and deallocation `O(1)`.
struct ReferenceCounters {
	/// The start of pages with free slots.
	free: Option<NonNull<ReferenceCountersTable>>,
	/// The start of pages that are full.
	full: Option<NonNull<ReferenceCountersTable>>,
}

/// Structure that represents either a counter or an offset.
union CounterOrOffset {
	counter: AtomicU16,
	offset: Option<NonZeroU16>,
}

/// Structure used to hold reference counters.
#[repr(C)]
struct ReferenceCountersTable {
	/// Pointer to the `next` field of the previous table **or** the `free` or `full` field of the
	/// `ReferenceCounters`.
	prev: Option<NonNull<Option<NonNull<ReferenceCountersTable>>>>,
	/// Pointer to the next table.
	next: Option<NonNull<ReferenceCountersTable>>,
	/// Offset of the next free counter.
	free: Option<NonZeroU16>,
	/// The counters.
	counters: [
		CounterOrOffset;
		(PAGE_SIZE - (
			2 * mem::size_of::<Option<NonNull<ReferenceCountersTable>>>() +
			mem::size_of::<Option<NonZeroU16>>()
		)) / mem::size_of::<AtomicU16>()
	],
}

const _SIZE_CHECK: usize = 0 - (PAGE_SIZE - mem::size_of::<ReferenceCountersTable>());

impl SharedPage {
	/// Create a new shared page.
	pub unsafe fn new(page: NonNull<Page>) -> Result<Self, AllocateError> {
		Ok(Self {
			page,
			counter: COUNTERS.lock().allocate()?,
		})
	}

	/// Attempt to increase the reference count of this page. It may fail if the counter would
	/// overflow.
	pub fn try_clone(&self) -> Result<Self, ReferenceCountOverflow> {
		// SAFETY: The pointer is valid: it was valid at the time of allocation and
		// it cannot have been freed yet (otherwise this function couldn't be called).
		let c = unsafe { self.counter.as_ref() };
		// Use CAS so that we can check for overflow.
		loop {
			let curr = c.load(Ordering::Relaxed);
			if let Some(new) = curr.checked_add(1) {
				if c.compare_exchange_weak(curr, new, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
					break Ok(Self {
						page: self.page,
						counter: self.counter,
					});
				}
			} else {
				break Err(ReferenceCountOverflow);
			}
		}
	}
}

impl Drop for SharedPage {
	fn drop(&mut self) {
		// SAFETY: The pointer is valid: it was valid at the time of allocation and
		// it cannot have been freed yet (otherwise this function couldn't be called).
		let c = unsafe { self.counter.as_ref() };
		// Underflow cannot happen unless we're the only owner, so fetch_sub can safely be used.
		// Note that fetch_sub returns the value from _before_ the substraction.
		let prev = c.fetch_sub(1, Ordering::Relaxed);
		if prev == 0 {
			// Free the counter and the page.
			// SAFETY: there is nothing else accessing the area, else we couldn't have been
			// dropped.
			unsafe { mem_deallocate(crate::memory::Area::new(self.page, 0).unwrap()).unwrap() };
			// SAFETY: only we own the counter
			unsafe { COUNTERS.lock().deallocate(self.counter) };
		}
	}
}

impl ReferenceCounters {
	fn allocate(&mut self) -> Result<NonNull<AtomicU16>, AllocateError> {
		let mut table = match self.free {
			Some(tbl) => tbl,
			None => {
				let mut tbl = mem_allocate(0)?.start().cast::<ReferenceCountersTable>();
				unsafe {
					tbl.as_ptr().write(ReferenceCountersTable {
						prev: NonNull::new(&mut self.free as *mut _),
						next: None,
						free: None,
						counters: mem::MaybeUninit::uninit().assume_init(),
					});
					let tbl = tbl.as_mut();
					tbl.free = Some(tbl.offset(NonNull::from(&tbl.counters[0])));
					for i in 0..tbl.counters.len() - 1 {
						tbl.counters[i].offset = Some(tbl.offset(NonNull::from(&tbl.counters[i + 1])));
					}
					tbl.counters.last_mut().unwrap().offset = None;
				}
				self.free = Some(tbl);
				tbl
			}
		};
		unsafe {
			let tbl = table.as_mut();
			let counter = tbl.allocate();
			if tbl.is_full() {
				self.free = tbl.next;
				self.full.as_mut().map(|t| t.as_mut().prev = tbl.prev);
				tbl.next = self.full;
				self.full = Some(table);
			}
			Ok(counter)
		}
	}

	unsafe fn deallocate(&mut self, counter: NonNull<AtomicU16>) {
		// SAFETY: The counter points to somewhere inside a page-aligned table,
		// so masking the lower bits will give us a pointer to the table itself.
		let table = (counter.as_ptr() as usize & !PAGE_MASK) as *mut ReferenceCountersTable;
		(&mut *table).deallocate(counter);
	}
}

impl ReferenceCountersTable {
	unsafe fn allocate(&mut self) -> NonNull<AtomicU16> {
		#[cfg(debug_assertions)]
		let offset = self.free.unwrap();
		#[cfg(not(debug_assertions))]
		let offset = self.free.unwrap_unchecked();
		let counter = ((self as *mut _ as usize) + usize::from(offset.get())) as *mut _;
		let mut counter = NonNull::<CounterOrOffset>::new_unchecked(counter);
		self.free = counter.as_ref().offset;
		counter.as_mut().offset = None;
		counter.cast()
	}

	unsafe fn deallocate(&mut self, counter: NonNull<AtomicU16>) {
		counter.cast::<Option<NonZeroU16>>().as_ptr().write(self.free);
		self.free = Some(self.offset(counter.cast()));
	}

	#[must_use]
	fn is_full(&self) -> bool {
		self.free.is_none()
	}

	#[must_use]
	unsafe fn offset(&self, counter: NonNull<CounterOrOffset>) -> NonZeroU16 {
		NonZeroU16::new_unchecked((counter.as_ptr() as usize - self as *const _ as usize) as u16)
	}
}

#[cfg(test)]
mod test {
	use super::*;

	test!(alloc_drop() {
		let page = mem_allocate(0).unwrap().start();
		unsafe {
			SharedPage::new(page).unwrap();
		}
	});

	test!(alloc_clone_drop() {
		let page = mem_allocate(0).unwrap().start();
		let page = unsafe {
			SharedPage::new(page).unwrap()
		};
		let page = page.try_clone().unwrap();
	});
}
