//! # Internal helper functions.
//!
//! These functions are not meant to be exposed. Instead, they reduce some boilerplate internally.

use core::mem;
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

impl_atomic!(AtomicBool, bool);
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
	let mut lock = SpinLockGuard::new(variable, value);
	// Perform whatever operations while locked
	f(&mut lock.value)
}

/// A guard for a spin lock.
pub struct SpinLockGuard<'a, T>
where
	T: Atomic,
{
	/// The actual lock.
	lock: &'a T,
	/// The value that should be put in the lock on release.
	///
	/// This *SHOULD* not match that of the original lock value.
	pub value: T::Inner,
}

#[derive(Debug)]
pub struct Locked;

impl<'a, T> SpinLockGuard<'a, T>
where
	T: Atomic,
{
	/// Attempt to store a certain value in an atomic variable and call the closure with the original.
	///
	/// If the original value is equal to the given value, it will try again until it no longer matches.
	///
	/// This function uses `Ordering::Acquire` on lock and `Ordering::Release` on release.
	pub fn new(lock: &'a T, value: T::Inner) -> Self {
		let mut val = lock.load(Ordering::Acquire);
		loop {
			// Wait until the lock is released.
			while val == value {
				val = lock.load(Ordering::Acquire);
			}

			// Try to get the lock
			match lock.compare_exchange_weak(val, value, Ordering::Acquire, Ordering::Acquire) {
				Ok(v) => return Self { lock, value: v },
				Err(v) => val = v,
			}
		}
	}

	/*
	/// Attempt to store a certain value in an atomic variable and call the closure with the original.
	///
	/// If the original value is equal to the given value, it will try again until it no longer matches.
	///
	/// This function uses `Ordering::Acquire` on lock and `Ordering::Release` on release.
	pub fn try_new(lock: &'a T, value: T::Inner) -> Result<Self, Locked> {
		// Try to get the lock
		let mut val = lock.load(Ordering::Acquire);
		if val != value {
			match lock.compare_exchange_weak(val, value, Ordering::Acquire, Ordering::Acquire) {
				Ok(v) => Ok(Self { lock, value: val })
				Err(_) => Err(Locked),
			}
		} else {
			Err(Locked)
		}
	}
	*/

	/// Consume the lock without releasing it & return the raw components.
	#[must_use]
	pub fn into_raw(self) -> (&'a T, T::Inner) {
		let lock_val = (self.lock, self.value);
		mem::forget(self);
		lock_val
	}

	/// Recreate a lock from it's raw components.
	///
	/// # Safety
	///
	/// The components **must** come from into_raw.
	pub unsafe fn from_raw(lock: &'a T, value: T::Inner) -> Self {
		Self { lock, value }
	}
}

impl<T> Drop for SpinLockGuard<'_, T>
where
	T: Atomic,
{
	fn drop(&mut self) {
		self.lock.store(self.value, Ordering::Release);
	}
}
