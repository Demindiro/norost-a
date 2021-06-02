use crate::arch::{Page, PAGE_SIZE};
use core::ptr::NonNull;

/// Structure representing a range of pages with a certain order.
#[derive(Clone, Copy, Debug)]
pub struct Area {
	/// The starting address of this area.
	start: NonNull<Page>,
	/// The order of this area
	order: u8,
}

/// Structure returned if an area isn't properly aligned.
#[derive(Debug)]
pub struct BadAlignment;

impl Area {
	/// Creates a new area of a given order and with a given address.
	pub fn new(start: NonNull<Page>, order: u8) -> Result<Self, BadAlignment> {
		(start.as_ptr() as usize & ((PAGE_SIZE << order) - 1) == 0)
			.then(|| Self { start, order })
			.ok_or(BadAlignment)
	}

	/// Creates a new area of a given order and with a given address without checking.
	///
	/// ## Safety
	///
	/// The area must uphold its variants.
	pub unsafe fn new_unchecked(start: NonNull<Page>, order: u8) -> Self {
		Self {
			start,
			order,
		}
	}

	/// Returns the start of this area
	pub fn start(&self) -> NonNull<Page> {
		self.start
	}

	/// Returns the order of this area
	pub fn order(&self) -> u8 {
		self.order
	}

	/// Split this area into two areas with half the size. 
	pub fn split(&self) -> Option<(Self, Self)> {
		self.order.checked_sub(1).map(|order| (
			Self {
				start: self.start,
				order,
			},
			Self {
				// SAFETY: the pointer won't overflow as that is a guarantee provided by Self
				// being a valid area.
				start: unsafe { NonNull::new_unchecked(self.start.as_ptr().add(1 << order)) },
				order,
			}
		))
	}
}
