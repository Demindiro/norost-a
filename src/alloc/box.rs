use core::alloc::{AllocError, Allocator, Layout};
use core::ptr::NonNull;
use core::{marker, mem, ptr};

/// See [`core::alloc::Box`](core::alloc::Box) for documentation.
//#[lang = "owned_box"]
pub struct Box<T, A>
where
	//T: ?Sized,
	A: Allocator,
{
	pointer: NonNull<T>,
	_marker: marker::PhantomData<T>,
	allocator: A,
}

impl<T, A> Box<T, A>
where
	//T: ?Sized,
	A: Allocator,
{
	pub fn try_new_in(value: T, allocator: A) -> Result<Self, AllocError> {
		let b = Self::try_new_uninit_in(allocator)?;
		let (mut ptr, alloc) = Box::into_raw_with_allocator(b);
		// SAFETY: we are only writing a value to unitialized memory. value is
		// always valid, hence transmuting from MaybeUninit<T> to T is safe.
		unsafe {
			ptr.as_mut().as_mut_ptr().write(value);
			Ok(Self::from_raw_in(ptr.cast(), alloc))
		}
	}

	pub fn try_new_uninit_in(allocator: A) -> Result<Box<mem::MaybeUninit<T>, A>, AllocError> {
		let layout = Layout::new::<T>();
		let ptr = allocator.allocate(layout)?;
		// SAFETY: the allocator returned a valid pointer
		unsafe { Ok(Box::from_raw_in(ptr.cast(), allocator)) }
	}

	pub fn try_new_zeroed_in(allocator: A) -> Result<Box<mem::MaybeUninit<T>, A>, AllocError> {
		let layout = Layout::new::<T>();
		let ptr = allocator.allocate_zeroed(layout)?;
		// SAFETY: the allocator returned a valid pointer
		unsafe { Ok(Box::from_raw_in(ptr.cast(), allocator)) }
	}

	pub unsafe fn from_raw_in(pointer: NonNull<T>, allocator: A) -> Self {
		Self {
			pointer,
			_marker: marker::PhantomData,
			allocator,
		}
	}

	pub fn into_raw_with_allocator(b: Self) -> (NonNull<T>, A) {
		let b = mem::ManuallyDrop::new(b);
		// SAFETY: The box won't be dropped as it is wrapped in a ManuallyDrop,
		// hence moving out of it is safe.
		let alloc = unsafe { ptr::read(&b.allocator) };
		(b.pointer, alloc)
	}

	pub fn leak<'a>(b: Self) -> &'a mut T {
		// SAFETY: not dropping the box is safe
		unsafe { mem::ManuallyDrop::new(b).pointer.as_mut() }
	}
}

impl<T, A> Drop for Box<T, A>
where
	//T: ?Sized,
	A: Allocator,
{
	fn drop(&mut self) {
		let layout = Layout::new::<T>();
		// SAFETY: self.pointer points to memory allocated by self.allocator
		unsafe {
			self.allocator.deallocate(self.pointer.cast(), layout);
		}
	}
}

/* TODO ????? I don't understand how this is supposed to be useable
 * https://doc.rust-lang.org/unstable-book/language-features/lang-items.html
#[lang = "box_free"]
unsafe fn box_free<T: ?Sized>(ptr: *mut T) {

}
*/
