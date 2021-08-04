use core::fmt;
use core::ptr::NonNull;

/// A pointer to a page.
///
/// The internal pointer is always properly aligned.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Page(NonNull<kernel::Page>);

impl Page {
	/// The size of a single page.
	pub const SIZE: usize = kernel::Page::SIZE;

	/// Create a new `Page` without runtime checks.
	///
	/// # Safety
	///
	/// The `Page` is properly aligned and not `null`.
	pub const unsafe fn new_unchecked(ptr: *mut kernel::Page) -> Self {
		// These allow the compiler to catch errors at compile time.
		//let _: usize = 0 - ptr.align_offset(kernel::Page::SIZE);
		let _: usize = 0 - (ptr as usize & (kernel::Page::SIZE - 1));
		let _: u8 = 0 - !ptr.is_null() as u8;
		Self(NonNull::new_unchecked(ptr))
	}

	/// Get the underlying pointer.
	pub const fn as_ptr(&self) -> *mut kernel::Page {
		self.0.as_ptr()
	}

	/// Get the underlying pointer.
	pub const fn as_non_null_ptr(&self) -> NonNull<kernel::Page> {
		self.0
	}

	/// Determine the minimum amount of pages needed to store the given amount of bytes.
	pub const fn min_pages_for_range(size: usize) -> usize {
		(size + Self::SIZE - 1) / Self::SIZE
	}
}

impl fmt::Debug for Page {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		stringify!(Page, "(").fmt(f)?;
		fmt::Pointer::fmt(self, f)?;
		")".fmt(f)
	}
}

impl fmt::Pointer for Page {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.0.fmt(f)
	}
}

impl fmt::Display for Page {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Pointer::fmt(self, f)
	}
}
