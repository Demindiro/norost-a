use crate::arch;
use core::convert::TryInto;
use core::fmt;
use core::mem;

pub(super) type PPNBox = u32;

/// A struct representing a PPN.
///
/// A PPN **cannot** be directly used as a physical address. It is formatted such that it doesn't
/// store the unneeded lower bits, which also allows using 32-bit PPNs on most 64-bit architecures.
pub struct PPN(PPNBox);

/// A struct representing a range of pages as PPNs.
pub struct PPNRange {
	start: PPNBox,
	count: u32,
}

impl PPN {
	/// Creates a new PPN from a physical pointer
	///
	/// ## Safety:
	///
	/// The pointer is aligned and within addressable range (`1 << 44` at most!).
	pub unsafe fn from_ptr(ptr: usize) -> Self {
		#[cfg(debug_assertions)]
		let p = {
			assert_eq!(ptr & arch::PAGE_MASK, 0, "Page pointer is not aligned");
			Self((ptr >> arch::PAGE_BITS).try_into().expect("PPN too large"))
		};
		#[cfg(not(debug_assertions))]
		let p = Self((ptr >> arch::PAGE_BITS) as u32);
		p
	}

	pub fn into_raw(self) -> u32 {
		let s = mem::ManuallyDrop::new(self);
		s.0
	}

	pub unsafe fn from_raw(ppn: u32) -> Self {
		Self(ppn)
	}
}

impl fmt::Debug for PPN {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "PPN (page: 0x{:x})", self.0 << 12)
	}
}

impl PPNRange {
	/// Creates a new PPN from a physical pointer and a count.
	///
	/// ## Safety:
	///
	/// The pointer is aligned and within addressable range (`1 << 44` at most!).
	pub unsafe fn from_ptr(start: usize, count: u32) -> Self {
		#[cfg(debug_assertions)]
		let start = {
			let start: u32 = (start >> arch::PAGE_BITS).try_into().expect("PPN too large");
			assert!(start.checked_add(count).is_some(), "start + count overflow");
			start
		};
		#[cfg(not(debug_assertions))]
		let start = (start >> arch::PAGE_BITS) as u32;
		Self { start, count }
	}

	/// Return the top PPN and decrement the count.
	pub fn pop(&mut self) -> Option<PPN> {
		self.count.checked_sub(1).map(|c| {
			self.count = c;
			PPN(self.start + c)
		})
	}

	/// Split off the top of this PPN.
	pub fn split(&mut self, count: u32) -> Option<Self> {
		self.count.checked_sub(count).map(|c| {
			self.count = c;
			Self {
				start: self.start + c,
				count,
			}
		})
	}

	/// The amount of pages this range spans.
	pub fn count(&self) -> u32 {
		self.count
	}
}

impl fmt::Debug for PPNRange {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		if self.count > 0 {
			let (s, e) = (self.start << 12, ((self.start + self.count) << 12) - 1);
			write!(f, "PPNRange (0x{:x}-0x{:x})", s, e)
		} else {
			write!(f, "PPNRange (empty)")
		}
	}
}
