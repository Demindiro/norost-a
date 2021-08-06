use core::convert::TryFrom;
/// A representation of a memory page.
///
/// This page is guaranteed to be properly aligned and non-null.
use core::fmt;
use core::num::NonZeroUsize;
use core::ptr::NonNull;

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Page(NonZeroUsize);

#[derive(Clone, Copy, Debug)]
pub struct BadAlignment;

impl Page {
	/// The size of the offset part in bits.
	pub const OFFSET_BITS: usize = 12;

	/// The size of a single page.
	pub const SIZE: usize = 4096;

	/// The alignment of a single page. This is always equivalent to the `SIZE`
	pub const ALIGN: usize = Self::SIZE;

	/// Create a new `Page` with the given aligned address.
	///
	/// Returns an error if the address isn't properly aligned.
	#[inline(always)]
	pub const fn new<T>(address: NonNull<T>) -> Result<Self, BadAlignment> {
		// Using transmute because fuck you rustc
		//
		//  error[E0133]: cast of pointer to int is unsafe and requires unsafe function or block
		//   --> kernel/src/memory/reserved.rs:53:20
		//    |
		// 53 |         let s = unsafe { self.end.as_ptr() as usize };
		//    |                          ^^^^^^^^^^^^^^^^^^^^^^^^^^ cast of pointer to int
		//    |
		//    = note: casting pointers to integers in constants
		use core::mem::transmute;
		// SAFETY: NonNull can never be casted to 0.
		let addr = unsafe { transmute::<_, NonZeroUsize>(address) };
		if addr.get() & (Self::ALIGN - 1) == 0 {
			Ok(Self(addr))
		} else {
			Err(BadAlignment)
		}
	}

	/// Create a new `Page` from a raw pointer.
	pub const fn from_pointer<T>(ptr: *mut T) -> Result<Self, FromPointerError> {
		// Why the fuck is NonNull::new not const?
		if !ptr.is_null() {
			let ptr = unsafe { NonNull::new_unchecked(ptr) };
			match Self::new(ptr) {
				Ok(page) => Ok(page),
				Err(BadAlignment) => Err(FromPointerError::BadAlignment),
			}
		} else {
			Err(FromPointerError::Null)
		}
	}

	/// Create a new `Page` from an usize.
	///
	/// Use this to avoid compiler weirdness because const fns are poorly supported.
	pub const fn from_usize(ptr: usize) -> Result<Self, FromPointerError> {
		if ptr != 0 {
			if ptr & (Self::ALIGN - 1) == 0 {
				Ok(unsafe { Self(NonZeroUsize::new_unchecked(ptr)) })
			} else {
				Err(FromPointerError::BadAlignment)
			}
		} else {
			Err(FromPointerError::Null)
		}
	}

	/// Return the address of this page.
	#[inline(always)]
	pub const fn as_ptr<T>(&self) -> *mut T {
		self.0.get() as *mut T
	}

	/// Return the address of this page.
	#[inline(always)]
	pub const fn as_non_null_ptr<T>(&self) -> NonNull<T> {
		// SAFETY: NonZeroUsize can never be casted to a null pointer
		unsafe { NonNull::new_unchecked(self.0.get() as *mut T) }
	}

	/// Return the page as an immutable reference.
	///
	/// # Safety
	///
	/// The page must point to valid and initialized memory.
	#[inline(always)]
	pub const unsafe fn as_ref<'a, T>(&self) -> &'a T {
		&*self.as_ptr()
	}

	/// Return the page as a mutable reference.
	///
	/// # Safety
	///
	/// The page must point to valid and initialized memory.
	///
	/// The memory may not be referenced already.
	#[inline(always)]
	pub const unsafe fn as_mut<'a, T>(&mut self) -> &'a mut T {
		&mut *self.as_ptr()
	}

	/// Return the page that comes after this one.
	#[inline(always)]
	pub const fn next(&self) -> Option<Self> {
		// Ditto. Rust pls
		let v = self.0.get().wrapping_add(Self::SIZE);
		if v != 0 {
			Some(Self(unsafe { NonZeroUsize::new_unchecked(v) }))
		} else {
			None
		}
	}

	/// Return the page at the given offset.
	#[inline(always)]
	pub const fn skip(&self, offset: usize) -> Option<Self> {
		if let Some(v) = self.0.get().checked_add(Self::SIZE * offset) {
			if v != 0 {
				Some(Self(unsafe { NonZeroUsize::new_unchecked(v) }))
			} else {
				None
			}
		} else {
			None
		}
	}
}

impl fmt::Debug for Page {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "0x{:x}", self.0)
	}
}

impl fmt::Pointer for Page {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Debug::fmt(self, f)
	}
}

/// Error returned if converting a pointer to a page failed.
#[derive(Clone, Copy, Debug)]
pub enum FromPointerError {
	/// The pointer is null
	Null,
	/// The pointer isn't properly aligned,
	BadAlignment,
}

impl<T> TryFrom<*mut T> for Page {
	type Error = FromPointerError;

	/// Create a new `Page` with the given pointer
	fn try_from(ptr: *mut T) -> Result<Self, Self::Error> {
		Self::from_pointer(ptr)
	}
}
