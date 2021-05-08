use core::ops::Deref;
use core::cell::Cell;

pub struct Mutex<T> {
	value: T,
	lock: Cell<bool>,
}

pub struct MutexGuard<'a, T> {
	mutex: &'a Mutex<T>,
}

unsafe impl<T> Sync for Mutex<T> {}

// FIXME proper atomic sync implementation
impl<T> Mutex<T> {
	pub const fn new(value: T) -> Self {
		Self { value, lock: Cell::new(false) }
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

impl<T> Deref for MutexGuard<'_, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.mutex.value
	}
}
