use super::{RawVec, ReserveError};
use core::alloc::{AllocError, Allocator, Layout};
use core::marker::PhantomData;
/// A common abstraction for `Vec`-like structures (`Vec`, `VecDeque`, ...)
///
/// Note that it *doesn't* drop it's contents when deallocated.
///
/// Implementation based on [`alloc::raw_vec::RawVec`][std rawvec].
///
/// [std rawvec]: https://github.com/rust-lang/rust/blob/master/library/alloc/src/raw_vec.rs
use core::ptr::NonNull;
use core::{mem, ops};

pub struct Vec<T, A>
where
	A: Allocator,
{
	raw_vec: RawVec<T, A>,
	length: usize,
}

impl<T, A> Vec<T, A>
where
	A: Allocator,
{
	pub fn new_in(allocator: A) -> Self {
		Self {
			raw_vec: RawVec::new_in(allocator),
			length: 0,
		}
	}

	pub fn capacity(&self) -> usize {
		self.raw_vec.capacity()
	}

	pub fn len(&self) -> usize {
		self.length
	}

	pub fn get(&self, index: usize) -> Option<&T> {
		if index < self.len() {
			// SAFETY: we check if the index is in range.
			Some(unsafe { self.raw_vec.get(index) })
		} else {
			None
		}
	}

	pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
		if index < self.len() {
			// SAFETY: we check if the index is in range.
			Some(unsafe { self.raw_vec.get_mut(index) })
		} else {
			None
		}
	}

	pub fn try_push(&mut self, element: T) -> Result<&mut T, ReserveError> {
		if self.len() >= self.raw_vec.capacity() {
			let cap = self.raw_vec.capacity();
			self.raw_vec.try_reserve((cap * 3 / 2 + 1) - cap)?;
		}
		// SAFETY: the index is in range
		let index = unsafe { self.raw_vec.get_mut(self.len()) };
		self.length += 1;
		*index = element;
		Ok((index))
	}
}

impl<T, A> ops::Deref for Vec<T, A>
where
	A: Allocator,
{
	type Target = [T];

	fn deref(&self) -> &[T] {
		// SAFETY: All elements up to self.len() are initialized.
		unsafe { self.raw_vec.as_slice(self.len()) }
	}
}

impl<T, A> ops::DerefMut for Vec<T, A>
where
	A: Allocator,
{
	fn deref_mut(&mut self) -> &mut [T] {
		// SAFETY: All elements up to self.len() are initialized.
		unsafe { self.raw_vec.as_slice_mut(self.len()) }
	}
}
