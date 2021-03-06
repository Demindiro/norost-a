use core::fmt;
use core::mem;
use core::ptr::NonNull;

/// Error returned if an address isn't properly aligned.
#[derive(Debug)]
pub struct Unaligned;

/// A pointer to a page.
///
/// The internal pointer is always properly aligned.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Page(NonNull<kernel::Page>);

impl Page {
	/// The size of a single page.
	pub const SIZE: usize = kernel::Page::SIZE;

	/// The address of the NULL page, which is not a valid, accessible page.
	pub const NULL_PAGE: *mut kernel::Page = core::ptr::null_mut();

	/// The end address of the NULL page. This address is inclusive.
	pub const NULL_PAGE_END: *mut kernel::Page = (Self::SIZE - 1) as *mut _;

	/// The mask of the offset bits.
	pub const OFFSET_MASK: usize = Self::SIZE - 1;

	/// Try to create new `Page`.
	pub const fn new(ptr: NonNull<kernel::Page>) -> Result<Self, Unaligned> {
		// Fuck you rustc
		if unsafe { core::mem::transmute::<_, usize>(ptr) } & (Self::SIZE - 1) == 0 {
			Ok(Self(ptr))
		} else {
			Err(Unaligned)
		}
	}

	/// Create a new `Page` without runtime checks.
	///
	/// # Safety
	///
	/// The `Page` is properly aligned and not `null`.
	pub const unsafe fn new_unchecked(ptr: *mut kernel::Page) -> Self {
		// These allow the compiler to catch errors at compile time.
		//let _: usize = 0 - ptr.align_offset(kernel::Page::SIZE);
		let _: usize = 0 - (mem::transmute::<_, usize>(ptr) & (kernel::Page::SIZE - 1));
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

	/// Returns the end address of this page.
	pub const fn end(&self) -> NonNull<kernel::Page> {
		unsafe { NonNull::new_unchecked(self.0.as_ptr().cast::<u8>().add(Self::SIZE - 1).cast()) }
	}

	/// Determine the minimum amount of pages needed to store the given amount of bytes.
	pub const fn min_pages_for_range(size: usize) -> usize {
		(size + Self::SIZE - 1) / Self::SIZE
	}
}

impl fmt::Debug for Page {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.write_str(concat!(stringify!(Page), "("))?;
		fmt::Pointer::fmt(self, f)?;
		f.write_str(")")
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

/// RWX flags used on pages.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RWX {
	R = 0b001,
	W = 0b010,
	X = 0b100,
	RW = 0b011,
	RX = 0b101,
	RWX = 0b111,
}

impl fmt::Display for RWX {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::R => "R  ",
			Self::W => " W ",
			Self::X => "  X",
			Self::RW => "RW ",
			Self::RX => "R X",
			Self::RWX => "RWX",
		}
		.fmt(f)
	}
}

impl From<RWX> for u8 {
	fn from(rwx: RWX) -> Self {
		match rwx {
			RWX::R => 0b001,
			RWX::W => 0b010,
			RWX::X => 0b100,
			RWX::RW => 0b011,
			RWX::RX => 0b101,
			RWX::RWX => 0b111,
		}
	}
}
