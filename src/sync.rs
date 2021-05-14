use core::cell::{Cell, UnsafeCell};
use core::ops;

pub struct Mutex<T> {
	value: UnsafeCell<T>,
	lock: Cell<bool>,
}

pub struct MutexGuard<'a, T> {
	mutex: &'a Mutex<T>,
}

unsafe impl<T> Sync for Mutex<T> {}

// FIXME proper atomic sync implementation
impl<T> Mutex<T> {
	pub const fn new(value: T) -> Self {
		Self {
			value: UnsafeCell::new(value),
			lock: Cell::new(false),
		}
	}

	pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
		if self.lock.get() {
			None
		} else {
			self.lock.set(true);
			Some(MutexGuard { mutex: self })
		}
	}

	pub fn lock(&self) -> MutexGuard<'_, T> {
		while self.lock.get() {}
		self.lock.set(true);
		MutexGuard { mutex: self }
	}
}

impl<T> Drop for MutexGuard<'_, T> {
	fn drop(&mut self) {
		self.mutex.lock.set(false);
	}
}

impl<T> ops::Deref for MutexGuard<'_, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		unsafe { &*self.mutex.value.get() }
	}
}

impl<T> ops::DerefMut for MutexGuard<'_, T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe { &mut *self.mutex.value.get() }
	}
}
