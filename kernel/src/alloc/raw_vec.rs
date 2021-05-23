use core::alloc::{AllocError, Allocator, Layout};
use core::marker::PhantomData;
use core::mem;
/// A common abstraction for `Vec`-like structures (`Vec`, `VecDeque`, ...)
///
/// Note that it *doesn't* drop it's contents when deallocated.
///
/// Implementation based on [`alloc::raw_vec::RawVec`][std rawvec].
///
/// [std rawvec]: https://github.com/rust-lang/rust/blob/master/library/alloc/src/raw_vec.rs
use core::ptr::NonNull;

#[derive(Debug)]
pub enum ReserveError {
	CapacityOverflow,
	AllocError,
}

pub(super) struct RawVec<T, A>
where
	A: Allocator,
{
	pointer: NonNull<T>,
	_marker: PhantomData<T>,
	capacity: usize,
	allocator: A,
}

impl<T, A> RawVec<T, A>
where
	A: Allocator,
{
	pub fn new_in(allocator: A) -> Self {
		Self {
			pointer: NonNull::dangling(),
			_marker: PhantomData,
			capacity: if mem::size_of::<T>() == 0 {
				usize::MAX
			} else {
				0
			},
			allocator,
		}
	}

	pub fn capacity(&self) -> usize {
		if mem::size_of::<T>() == 0 {
			usize::MAX
		} else {
			self.capacity
		}
	}

	pub fn try_reserve(&mut self, additional: usize) -> Result<(), ReserveError> {
		let new_cap = self
			.capacity()
			.checked_add(additional)
			.ok_or(ReserveError::CapacityOverflow)?;
		let new_layout = Layout::array::<T>(new_cap)
			.ok()
			.ok_or(ReserveError::CapacityOverflow)?;
		let old_layout = Layout::array::<T>(self.capacity()).unwrap();
		// SAFETY: the pointer is valid
		unsafe {
			let ptr = self
				.allocator
				.grow(self.pointer.cast(), old_layout, new_layout)?;
			let (ptr, cap) = ptr.to_raw_parts();
			self.capacity = cap / Layout::new::<T>().pad_to_align().size();
			self.pointer = ptr.cast();
		}
		Ok(())
	}

	pub unsafe fn get(&self, index: usize) -> &T {
		debug_assert!(index < self.capacity);
		// SAFETY: the index is within range
		self.pointer.as_ptr().add(index).as_ref().unwrap_unchecked()
	}

	pub unsafe fn get_mut(&mut self, index: usize) -> &mut T {
		debug_assert!(index < self.capacity);
		// SAFETY: the index is within range
		self.pointer.as_ptr().add(index).as_mut().unwrap_unchecked()
	}

	pub unsafe fn as_slice(&self, count: usize) -> &[T] {
		debug_assert!(count <= self.capacity);
		// SAFETY: the index is within range
		NonNull::slice_from_raw_parts(self.pointer, count).as_ref()
	}

	pub unsafe fn as_slice_mut(&self, count: usize) -> &mut [T] {
		debug_assert!(count <= self.capacity);
		// SAFETY: the index is within range
		NonNull::slice_from_raw_parts(self.pointer, count).as_mut()
	}
}

impl From<AllocError> for ReserveError {
	fn from(error: AllocError) -> Self {
		Self::AllocError
	}
}
