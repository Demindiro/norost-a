//! # Internal helper functions.
//!
//! These functions are not meant to be exposed. Instead, they reduce some boilerplate internally.

use core::sync::atomic::*;

pub trait Atomic
where
	Self::Inner: Copy + Clone + PartialEq + Eq,
{
	type Inner;

	fn load(&self, ordering: Ordering) -> Self::Inner;

	fn store(&self, value: Self::Inner, ordering: Ordering);

	fn compare_exchange_weak(
		&self,
		current: Self::Inner,
		new: Self::Inner,
		success: Ordering,
		failure: Ordering,
	) -> Result<Self::Inner, Self::Inner>;
}

macro_rules! impl_atomic {
	($atomic:ty, $inner:ty) => {
		impl Atomic for $atomic {
			type Inner = $inner;

			fn load(&self, ordering: Ordering) -> Self::Inner {
				self.load(ordering)
			}

			fn store(&self, value: Self::Inner, ordering: Ordering) {
				self.store(value, ordering);
			}

			fn compare_exchange_weak(
				&self,
				current: Self::Inner,
				new: Self::Inner,
				success: Ordering,
				failure: Ordering,
			) -> Result<Self::Inner, Self::Inner> {
				self.compare_exchange_weak(current, new, success, failure)
			}
		}
	};
}

impl_atomic!(AtomicU8, u8);
impl_atomic!(AtomicU16, u16);
impl_atomic!(AtomicU32, u32);
impl_atomic!(AtomicUsize, usize);

/// Attempt to store a certain value in an atomic variable and call the closure with the original.
///
/// If the original value is equal to the given value, it will try again until it no longer matches.
///
/// This function uses `Ordering::Acquire` on lock and `Ordering::Release` on release.
pub(crate) fn spin_lock<T, F, R>(variable: &T, value: T::Inner, f: F) -> R
where
	T: Atomic,
	F: FnOnce(&mut T::Inner) -> R,
{
	// Try to get the lock
	let mut val = loop {
		let val = variable.load(Ordering::Relaxed);
		if val != value {
			if variable
				.compare_exchange_weak(val, value, Ordering::Acquire, Ordering::Relaxed)
				.is_ok()
			{
				break val;
			}
		}
	};
	// Perform whatever operations while locked
	let ret = f(&mut val);
	// Release the lock
	variable.store(val, Ordering::Release);
	ret
}
