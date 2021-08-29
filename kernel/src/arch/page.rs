use core::convert::TryFrom;
/// A representation of a memory page.
///
/// This page is guaranteed to be properly aligned and non-null.
use core::fmt;
use core::ptr::NonNull;

/// A representation of a raw page.
#[repr(align(4096))]
pub struct PageData([u8; Self::SIZE]);

impl PageData {
	/// The size of the offset part in bits.
	pub const OFFSET_BITS: usize = 12;

	/// The size of a single page.
	pub const SIZE: usize = 4096;

	/// The alignment of a single page. This is always equivalent to the `SIZE`
	pub const ALIGN: usize = Self::SIZE;

	/// The mask for the offset bits.
	pub const OFFSET_MASK: usize = (1 << Self::OFFSET_BITS) - 1;
}

/// The address of a page.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Page(NonNull<PageData>);

#[derive(Clone, Copy, Debug)]
pub struct BadAlignment;

impl Page {
	/// The size of the offset part in bits.
	#[allow(dead_code)]
	pub const OFFSET_BITS: usize = PageData::OFFSET_BITS;

	/// The size of a single page.
	pub const SIZE: usize = PageData::SIZE;

	/// The alignment of a single page. This is always equivalent to the `SIZE`
	pub const ALIGN: usize = PageData::ALIGN;

	/// The mask for the offset bits.
	pub const OFFSET_MASK: usize = PageData::OFFSET_MASK;

	/// Create a new `Page` with the given aligned address.
	///
	/// Returns an error if the address isn't properly aligned.
	#[inline(always)]
	pub const fn new(address: NonNull<PageData>) -> Result<Self, BadAlignment> {
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
		// SAFETY: usize can represent all possible values of address.
		let addr = unsafe { transmute::<_, usize>(address) };
		if addr & (Self::ALIGN - 1) == 0 {
			Ok(Self(address))
		} else {
			Err(BadAlignment)
		}
	}

	/// Create a new `Page` from a raw pointer.
	pub const fn from_pointer(ptr: *mut PageData) -> Result<Self, FromPointerError> {
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
				Ok(unsafe { Self(NonNull::new_unchecked(ptr as *mut _)) })
			} else {
				Err(FromPointerError::BadAlignment)
			}
		} else {
			Err(FromPointerError::Null)
		}
	}

	/// Return the address of this page.
	#[inline(always)]
	pub const fn as_ptr(&self) -> *mut PageData {
		self.0.as_ptr()
	}

	/// Return the address of this page.
	#[inline(always)]
	pub const fn as_non_null_ptr(&self) -> NonNull<PageData> {
		self.0
	}

	/// Return the page that comes after this one.
	#[inline(always)]
	pub const fn next(&self) -> Option<Self> {
		self.skip(1)
	}

	/// Return the page at the given offset.
	#[inline(always)]
	pub const fn skip(&self, offset: usize) -> Option<Self> {
		unsafe {
			let ptr = self.0.as_ptr().add(offset);
			if ptr.is_null() {
				None
			} else {
				Some(Self(NonNull::new_unchecked(ptr)))
			}
		}
	}

	/// Return the amount of pages needed to cover the given amount of bytes.
	#[inline(always)]
	pub const fn min_pages_for_byte_count(bytes: usize) -> usize {
		(bytes + Self::OFFSET_MASK) / Self::SIZE
	}
}

impl fmt::Debug for Page {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "Page({:p})", self.0)
	}
}

impl fmt::Pointer for Page {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Pointer::fmt(&self.0, f)
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

impl TryFrom<*mut PageData> for Page {
	type Error = FromPointerError;

	/// Create a new `Page` with the given pointer
	fn try_from(ptr: *mut PageData) -> Result<Self, Self::Error> {
		Self::from_pointer(ptr)
	}
}

/// Working around Rust retardedness that will probably never be fixed
unsafe impl Sync for Page {}
