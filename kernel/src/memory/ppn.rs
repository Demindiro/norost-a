use crate::arch;
use core::convert::TryInto;
use core::fmt;

/// A struct representing a PPN.
///
/// A PPN **cannot** be directly used as a physical address. It is formatted such that it doesn't
/// store the unneeded lower bits, which also allows using 32-bit PPNs on most 64-bit architecures.
#[derive(Clone, Copy)]
pub struct PPN(u32);

/// A struct representing a range of pages as PPNs.
pub struct PPNRange {
	start: PPN,
	count: u32,
}

impl PPN {
	pub fn new(ptr: usize) -> Self {
		#[cfg(debug_assertions)]
		let p = {
			assert_eq!(ptr & arch::PAGE_MASK, 0, "Page pointer is not aligned");
			Self((ptr >> arch::PAGE_BITS).try_into().expect("PPN too large"))
		};
		#[cfg(not(debug_assertions))]
		let p = Self((ptr >> arch::PAGE_BITS) as u32);
		p
	}
}

impl From<PPN> for usize {
	fn from(ppn: PPN) -> Self {
		ppn.0.try_into().unwrap()
	}
}

impl fmt::Debug for PPN {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "PPN (page: 0x{:x})", self.0 << 12)
	}
}

impl PPNRange {
	pub fn new(start: usize, count: u32) -> Self {
		let p = PPN::new(start);
		#[cfg(debug_assertions)]
		assert!(p.0.checked_add(count).is_some(), "start + count overflow");
		Self {
			start: p,
			count,
		}
	}

	/// Return the top PPN and decrement the count.
	pub fn pop(&mut self) -> Option<PPN> {
		self.count.checked_sub(1).map(|c| {
			self.count = c;
			PPN(self.start.0 + c)
		})
	}

	/// Split off the top of this PPN.
	pub fn split(&mut self, count: u32) -> Option<Self> {
		self.count.checked_sub(count).map(|c| {
			self.count = c;
			Self {
				start: PPN(self.start.0 + c),
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
			let (s, e) = (self.start.0 << 12, ((self.start.0 + self.count) << 12) - 1);
			write!(f, "PPNRange (0x{:x}-0x{:x})", s, e)
		} else {
			write!(f, "PPNRange (empty)")
		}
	}
}
