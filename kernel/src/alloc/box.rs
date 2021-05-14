use core::alloc::{AllocError, Allocator, Layout};
use core::ptr::NonNull;
use core::{convert, marker, mem, ops, ptr};

/// See [`core::alloc::Box`](core::alloc::Box) for documentation.
// Lang items aren't used atm because of an ICE (and my mistrusting ass
// has a hunch that the response I'll get is "don't use that!!!",
// as happens with too many projects...).
//#[lang = "owned_box"]
pub struct Box<T, A>
where
	T: ?Sized,
	A: Allocator,
{
	pointer: NonNull<T>,
	_marker: marker::PhantomData<T>,
	allocator: A,
}

impl<T, A> Box<T, A>
where
	T: ?Sized,
	A: Allocator,
{
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

impl<T, A> Box<T, A>
where
	T: Sized,
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

	pub unsafe fn assume_init(self) -> Self {
		let (ptr, alloc) = Box::into_raw_with_allocator(self);
		Self::from_raw_in(ptr.cast(), alloc)
	}
}

impl<T, A> Box<[T], A>
where
	T: Sized,
	A: Allocator,
{
	/// Attempts to create a boxed slice from an
	/// [`ExactSizeIterator`](core::iter::ExactSizeIterator).
	pub fn try_from_iterator<I>(iterator: I, allocator: A) -> Result<Self, AllocError>
	where
		I: ExactSizeIterator<Item = T>,
	{
		let mut b = Self::try_new_uninit_slice_in(iterator.len(), allocator)?;
		for (i, e) in iterator.enumerate() {
			b[i].write(e);
		}
		// SAFETY: we initialized all elements
		Ok(unsafe { b.assume_init() })
	}

	pub fn try_new_uninit_slice_in(
		size: usize,
		allocator: A,
	) -> Result<Box<[mem::MaybeUninit<T>], A>, AllocError> {
		let layout = Layout::new::<T>().repeat(size).ok().ok_or(AllocError)?.0;
		let ptr = allocator.allocate(layout)?;
		let ptr = NonNull::slice_from_raw_parts(ptr.cast(), size);
		// SAFETY: the allocator returned a valid pointer
		unsafe { Ok(Box::from_raw_in(ptr, allocator)) }
	}

	pub fn try_new_zeroed_slice_in(
		size: usize,
		allocator: A,
	) -> Result<Box<[mem::MaybeUninit<T>], A>, AllocError> {
		let layout = Layout::new::<T>().repeat(size).ok().ok_or(AllocError)?.0;
		let ptr = allocator.allocate_zeroed(layout)?;
		let ptr = NonNull::slice_from_raw_parts(ptr.cast(), size);
		// SAFETY: the allocator returned a valid pointer
		unsafe { Ok(Box::from_raw_in(ptr, allocator)) }
	}
}

impl<T, A> Box<[mem::MaybeUninit<T>], A>
where
	T: Sized,
	A: Allocator,
{
	pub unsafe fn assume_init(self) -> Box<[T], A> {
		let (ptr, alloc) = Box::into_raw_with_allocator(self);
		let ptr = ptr.as_ptr() as *mut _ as *mut [T];
		Box::from_raw_in(NonNull::new_unchecked(ptr), alloc)
	}
}

impl<A> Box<str, A>
where
	A: Allocator,
{
	/// Attempts to create a boxed string from an [`str`](str).
	pub fn try_from_str(string: &str, allocator: A) -> Result<Self, AllocError> {
		let b = Box::try_from_iterator(string.bytes(), allocator)?;
		let (ptr, alloc) = Box::into_raw_with_allocator(b);
		let (ptr, _) = ptr.to_raw_parts();
		let ptr = NonNull::from_raw_parts(ptr, string.len());
		// SAFETY: string is already valid, so casting from &[u8] to &str is safe.
		unsafe { Ok(Self::from_raw_in(ptr, alloc)) }
	}
}

impl<T, A> Drop for Box<T, A>
where
	T: ?Sized,
	A: Allocator,
{
	fn drop(&mut self) {
		// SAFETY: self.pointer points to memory allocated by self.allocator and is
		// valid/initialized.
		unsafe {
			let layout = Layout::for_value(self.pointer.as_ref());
			ptr::drop_in_place(self.pointer.as_ptr());
			self.allocator.deallocate(self.pointer.cast(), layout);
		}
	}
}

impl<T, A> ops::Deref for Box<T, A>
where
	T: ?Sized,
	A: Allocator,
{
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.as_ref()
	}
}

impl<T, A> ops::DerefMut for Box<T, A>
where
	T: ?Sized,
	A: Allocator,
{
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.as_mut()
	}
}

impl<T, A> AsRef<T> for Box<T, A>
where
	T: ?Sized,
	A: Allocator,
{
	fn as_ref(&self) -> &T {
		// SAFETY: self.pointer points to memory allocated by self.allocator
		unsafe { self.pointer.as_ref() }
	}
}

impl<T, A> AsMut<T> for Box<T, A>
where
	T: ?Sized,
	A: Allocator,
{
	fn as_mut(&mut self) -> &mut T {
		// SAFETY: self.pointer points to memory allocated by self.allocator
		unsafe { self.pointer.as_mut() }
	}
}

/* TODO ????? I don't understand how this is supposed to be useable
 * https://doc.rust-lang.org/unstable-book/language-features/lang-items.html
#[lang = "box_free"]
unsafe fn box_free<T: ?Sized>(ptr: *mut T) {

}
*/
