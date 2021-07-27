//! Management of shared pages.

use super::reserved::{SHARED_ALLOC, SHARED_COUNTERS};
use super::{AllocateError, PPNBox, PPN};
use crate::arch::PAGE_BITS;
use core::fmt;
use core::mem;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicI16, AtomicU32, Ordering};

const COUNTERS: NonNull<AtomicU32> = SHARED_COUNTERS.start.as_non_null_ptr();
const ALLOC: NonNull<AtomicI16> = SHARED_ALLOC.start.as_non_null_ptr();

/// Representation of a physical page that can be safely shared.
pub struct SharedPPN(u32);

/// Structure returned if the reference count would overflow.
#[derive(Debug)]
pub struct ReferenceCountOverflow;

impl SharedPPN {
	/// Create a new shared page.
	pub fn new(ppn: PPN) -> Result<Self, AllocateError> {
		let ppn = ppn.into_raw();
		// Ensure the underlying page is allocated.
		let counter = unsafe { &mut *ALLOC.as_ptr().add(ppn as usize >> PAGE_BITS) };
		loop {
			// Try to get an allocation lock.
			match counter.compare_exchange_weak(0, -1, Ordering::Relaxed, Ordering::Relaxed) {
				// We got the lock and need to allocate
				Ok(_) => {
					todo!()
				}
				// Another hart is already allocating, so try again.
				// Retrying is necessary in case the hart drops the page immediately after,
				// requiring an allocation regardless.
				Err(-1) => (),
				// The page is already allocated, so try to increase.
				// Trying is necessary in case the page just got dropped.
				Err(c) => {
					if counter
						.compare_exchange_weak(c, c + 1, Ordering::Relaxed, Ordering::Relaxed)
						.is_err()
					{
						break;
					}
				}
			}
		}
		// Set counter to 0 from -1 or any garbage value.
		let counter = unsafe { &mut *COUNTERS.as_ptr().add(ppn as usize) };
		counter.store(0, Ordering::Relaxed);

		Ok(Self(ppn))
	}

	/// Attempt to increase the reference count of this page. It may fail if the counter would
	/// overflow.
	#[allow(dead_code)]
	pub fn try_clone(&self) -> Result<Self, ReferenceCountOverflow> {
		// SAFETY: The pointer is valid: it was valid at the time of allocation and
		// it cannot have been freed yet (otherwise this function couldn't be called).
		let counter = unsafe { &mut *COUNTERS.as_ptr().add(self.0 as usize) };
		// Use CAS so that we can check for overflow.
		loop {
			let curr = counter.load(Ordering::Relaxed);
			if let Some(new) = curr.checked_add(1) {
				if counter
					.compare_exchange_weak(curr, new, Ordering::Relaxed, Ordering::Relaxed)
					.is_ok()
				{
					break Ok(Self(self.0));
				}
			} else {
				break Err(ReferenceCountOverflow);
			}
		}
	}

	/// Return the PPN of this page. This does not decrement the reference count.
	pub fn into_raw(self) -> PPN {
		let s = mem::ManuallyDrop::new(self);
		// SAFETY: The PPN is valid as only we own it and thus can't have
		// been freed by something else.
		unsafe { PPN::from_raw(s.0) }
	}

	/// Create a `SharedPPN` from the PPN.
	///
	/// ## Safety
	///
	/// The PPN did originally come from a `SharedPPN`
	pub unsafe fn from_raw(ppn: PPN) -> Self {
		let ppn = ppn.into_raw();
		Self(ppn)
	}
}

impl Drop for SharedPPN {
	fn drop(&mut self) {
		// SAFETY: The pointer is valid: it was valid at the time of allocation and
		// it cannot have been freed yet (otherwise this function couldn't be called).
		let counter = unsafe { &mut *COUNTERS.as_ptr().add(self.0 as usize) };
		// Underflow cannot happen unless we're the only owner, so fetch_sub can safely be used.
		// Note that fetch_sub returns the value from _before_ the substraction.
		if counter.fetch_sub(1, Ordering::Relaxed) == 0 {
			// Free the page.
			let _ = unsafe { PPN::from_raw(self.0) };
			// Free the counter
			todo!();
		}
	}
}

impl fmt::Debug for SharedPPN {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		// SAFETY: The pointer is valid: it was valid at the time of allocation and
		// it cannot have been freed yet (otherwise this function couldn't be called).
		let counter = unsafe { &mut *COUNTERS.as_ptr().add(self.0 as usize) };
		// Underflow cannot happen unless we're the only owner, so fetch_sub can safely be used.
		// Note that fetch_sub returns the value from _before_ the substraction.
		let counter = counter.load(Ordering::Relaxed);
		write!(f, "SharedPPN (page: 0x{:x}, count: {})", self.0, counter)
	}
}

pub struct SharedPPNRange {
	_m: (),
}

impl SharedPPNRange {
	pub fn len(&self) -> usize {
		todo!()
	}

	#[must_use]
	pub fn start(&self) -> PPNBox {
		todo!()
	}

	pub fn pop_base(&mut self) -> Option<SharedPPN> {
		todo!()
	}

	/// Forget about the last N PPNs and return the amount of PPNs that actually got removed.
	#[must_use]
	#[track_caller]
	#[inline]
	pub fn forget_base(&mut self, count: usize) -> usize {
		todo!()
	}
}

#[cfg(test)]
mod test {
	use super::*;

	fn reset() {}

	test!(alloc_drop() {
		reset();
		let page = mem_allocate(0).unwrap().start();
		unsafe {
			SharedPPN::new(page).unwrap();
		}
	});

	test!(alloc_clone_drop() {
		reset();
		let page = mem_allocate(0).unwrap().start();
		let page = unsafe {
			SharedPPN::new(page).unwrap()
		};
		let page = page.try_clone().unwrap();
	});

	test!(alloc_into_raw_parts() {
		reset();
		let page = mem_allocate(0).unwrap().start();
		let page = unsafe { SharedPPN::new(page).unwrap() };
		let (page, counter) = page.into_raw_parts();
		let page = unsafe { SharedPPN::from_raw_parts(page, counter) };
	});
}
