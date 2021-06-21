use crate::arch;
use core::convert::{TryFrom, TryInto};
use core::fmt;
use core::mem;

pub use super::{SharedPPN, SharedPPNRange};

pub type PPNBox = u32;

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

	/// Only use this function if the page is identity mapped!
	pub unsafe fn as_ptr(&self) -> *mut arch::Page {
		((self.0 as usize) << 12) as *mut _
	}

	pub fn as_usize(&self) -> usize {
		(self.0 as usize) << 12
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

	/// Return the bottom PPN and decrement the count.
	pub fn pop_base(&mut self) -> Option<PPN> {
		self.count.checked_sub(1).map(|c| {
			self.count = c;
			let ppn = PPN(self.start);
			self.start += 1;
			ppn
		})
	}

	/// Return the start address of this range as a usize
	pub fn as_usize(&self) -> usize {
		(self.start as usize) << 12
	}

	/// Forget about the last N PPNs and return the amount of PPNs that actually got removed.
	#[must_use]
	#[track_caller]
	#[inline]
	pub fn forget_base(&mut self, count: usize) -> usize {
		use core::convert::TryInto;
		let count = count.try_into().unwrap();
		if let Some(c) = self.count.checked_sub(count) {
			self.count = c;
			self.start += count;
			count
		} else {
			core::mem::replace(&mut self.count, 0)
		}.try_into().unwrap()
	}

	/// Return the amount of pages this range spans.
	#[must_use]
	pub fn len(&self) -> usize {
		self.count as usize
	}

	#[must_use]
	pub fn start(&self) -> PPNBox {
		self.start
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

impl PPNDirect {
	#[inline]
	#[must_use]
	pub fn from_usize(ptr: usize) -> Result<Self, Error> {
		if ptr & arch::PAGE_MASK > 0 {
			Err(Error::BadAlignment)
		} else if let Ok(ppn) = (ptr >> arch::PAGE_BITS).try_into() {
			Ok(Self(ppn))
		} else {
			Err(Error::OutOfRange)
		}
	}
}

/// A page that was allocated directly and hence is not tracked by the PMM.
pub struct PPNDirect(PPNBox);

impl From<PPNDirect> for PPNBox {
	fn from(ppn: PPNDirect) -> PPNBox {
		ppn.0
	}
}

impl From<PPNBox> for PPNDirect {
	fn from(ppn: PPNBox) -> PPNDirect {
		Self(ppn)
	}
}

impl fmt::Debug for PPNDirect {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "PPNDirect (page: 0x{:x})", self.0 << 12)
	}
}

/// A range of pages that were allocated directly and hence aren't tracked by the PPM.
pub struct PPNDirectRange {
	start: PPNBox,
	count: u32,
}

impl PPNDirectRange {
	/// Create a new direct PPN range.
	pub fn new(start: PPNBox, count: usize) -> Result<Self, Error> {
		let count = u32::try_from(count).map_err(|_| Error::OutOfRange)?;
		Ok(Self { start, count })
	}

	/// Return the bottom PPN and decrement the count.
	pub fn pop_base(&mut self) -> Option<PPNDirect> {
		self.count.checked_sub(1).map(|c| {
			self.count = c;
			let ppn = PPNDirect(self.start);
			self.start += 1;
			ppn
		})
	}

	pub fn len(&self) -> usize {
		self.count.try_into().unwrap()
	}

	#[must_use]
	pub fn start(&self) -> PPNBox {
		self.start
	}

	/// Forget about the last N PPNs and return the amount of PPNs that actually got removed.
	#[must_use]
	#[track_caller]
	#[inline]
	pub fn forget_base(&mut self, count: usize) -> usize {
		use core::convert::TryInto;
		let count = count.try_into().unwrap();
		if let Some(c) = self.count.checked_sub(count) {
			self.count = c;
			self.start += count;
			count
		} else {
			core::mem::replace(&mut self.count, 0)
		}.try_into().unwrap()
	}
}

impl fmt::Debug for PPNDirectRange {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		if self.count > 0 {
			let (s, e) = (self.start << 12, ((self.start + self.count) << 12) - 1);
			write!(f, "PPNDirectRange (0x{:x}-0x{:x})", s, e)
		} else {
			write!(f, "PPNDirectRange (empty)")
		}
	}
}

pub enum Error {
	BadAlignment,
	OutOfRange,
}

impl fmt::Debug for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.write_str(match self {
			Self::BadAlignment => "Bad alignment",
			Self::OutOfRange => "Out of range",
		})
	}
}
