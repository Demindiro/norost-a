//! # Buddy allocator
//!
//! This module keeps track of free pages in a given range of memory.
//!
//! The "Binary Buddy Algorithm" is used as it's very simple yet efficient.
//!
//! ## References
//!
//! [Physical Memory Allocation][buddy]
//!
//! [buddy]: https://www.kernel.org/doc/gorman/html/understand/understand009.html

pub use crate::arch::{Page, PAGE_SIZE};
use super::Area;
use crate::sync::Mutex;
use core::num::NonZeroUsize;
use core::ptr::{self, NonNull};
use core::{mem, slice};

pub struct BuddyAllocator<const O: usize>
where
	[(); O + 1]: Sized,
{
	/// A list of free areas, sorted by size (i.e. order)
	///
	/// The order determines the size of each area, expressed in pages. Each area is
	/// `PAGE_SIZE * (1 << order)` large.
	// TODO 'static is probably not the right thing to use, but I'm not sure how else it should
	// be done.
	free_areas: [FreeAreaList<'static>; O + 1],
	/// The start of the managed zone. Used for calculating bitmask offsets and range checks.
	start: NonNull<Page>,
	/// The end of the managed zone. Used for range checks.
	/// This is **inclusive** (i.e. the actual address is actually `start + page_count - 1`) so
	/// that it can't be a null pointer in case we manage all the memory at the top (e.g. 32 bit
	/// systems with 4GiB memory). This also means this page isn't properly aligned.
	end: NonNull<Page>,
	// TODO tracking information (trackers can be used to catch bad VMM behaviour)
	#[cfg(debug_assertions)]
	last_free_addr: Option<NonNull<Page>>,
}

unsafe impl<const O: usize> Sync for BuddyAllocator<O>
where
	[(); O + 1]: Sized,
{}

/// Structure to keep track of free areas
struct FreeAreaList<'a> {
	/// The head of a linked list of free areas.
	next: Option<NonNull<FreeArea>>,
	/// A bitmap to keep track of free buddies.
	///
	/// A buddy is a set of two areas of the same order.
	///
	/// * 0 means either both are freed or both are in use.
	/// * 1 means one of both is in use and the other is free.
	buddies_state: &'a mut [usize],
}

/// Structure representing a free area
struct FreeArea {
	/// The next free area of the same size, if any.
	next: Option<NonNull<FreeArea>>,
}

/// Enum representing errors that can occur when trying to allocate an area.
#[derive(Debug, PartialEq)]
pub enum AllocateError {
	/// There are no more areas of the requested size left to hand out.
	Empty,
	/// The requested order is larger than the maximum order.
	OrderTooLarge,
}

/// Enum representing errors that can occur when trying to deallocate an area.
#[derive(Debug, PartialEq)]
pub enum DeallocateError {
	/// The area is not in the range handled by the manager.
	OutOfBounds,
	/// The tracker tag doesn't match.
	///
	/// This is useful to detect and protect against misbehaving VMMs.
	TrackerMismatch,
	/// The passed order is larger than the maximum order.
	OrderTooLarge,
}

const USIZE_BITS: usize = mem::size_of::<usize>() * 8;
const USIZE_MASK: usize = mem::size_of::<usize>() * 8 - 1;

impl<const O: usize> BuddyAllocator<O>
where
	[(); O + 1]: Sized,
{
	/// Creates a new `MemoryBuddyAllocator` with governance over the given range of pages.
	///
	/// ## Safety
	///
	/// The memory range must be unused.
	#[must_use]
	pub unsafe fn new(start: NonNull<Page>, count: NonZeroUsize) -> Self {
		// Clear all pages
		for i in 0..count.get() {
			start.as_ptr().add(i).as_mut().unwrap_unchecked().clear();
		}

		let mut count = count.get();

		// The total amount of bits needed, which is ceil(count / 2) + ceil(count / 4) + ...
		// + ceil(count >> (O + 1)).
		// There is probably a concise way (i.e. non-iterative) to express this but w/e.
		let mut bitmap_size = 0;
		for o in 0..O {
			// (count + 1) / 2^n is effectively ceil(count / 2^n)
			let size = (count + 1) >> o;
			let size = (size + USIZE_MASK) & !USIZE_MASK;
			let size = size / (USIZE_MASK + 1);
			bitmap_size += size;
		}

		// Express the size in the upper bound of usizes and pages needed
		let bitmap_size_pages = (bitmap_size + (PAGE_SIZE - 1)) & !(PAGE_SIZE - 1);
		let bitmap_size_pages = bitmap_size_pages / PAGE_SIZE;
		if O > 0 {
			debug_assert_ne!(bitmap_size, 0, "bitmap size cannot be zero");
			debug_assert_ne!(bitmap_size_pages, 0, "bitmap size in pages cannot be zero");
		} else {
			debug_assert_eq!(bitmap_size, 0, "bitmap size must be zero");
			debug_assert_eq!(bitmap_size_pages, 0, "bitmap size in pages must be zero");
		}

		// Create the bitmap, clear it and bump up start and count appropriately
		let bitmap = start.cast().as_ptr();
		let mut bitmap = slice::from_raw_parts_mut(bitmap, bitmap_size);
		bitmap.fill(0);

		// SAFETY: start won't overflow
		let start = NonNull::new_unchecked(start.as_ptr().add(bitmap_size_pages));
		count -= bitmap_size_pages;

		let mut areas = mem::MaybeUninit::uninit_array::<{ O + 1 }>();
		for i in 0..O {
			let count @ offset = count >> i;
			// Take a piece of the full bitmap
			let offset = (offset + USIZE_MASK) & !USIZE_MASK;
			let offset = offset / (USIZE_MASK + 1);
			let (left, right) = bitmap.split_at_mut(offset);
			// If the number of areas is odd, set the last bit to 1 to pretend it's buddy
			// is allocated forever.
			if count % 2 > 0 {
				let offset = (count / 2) & USIZE_MASK;
				*left.last_mut().unwrap() = 1 << offset;
			}
			// Create a list
			areas[i].write(FreeAreaList {
				next: None,
				buddies_state: left,
			});
			bitmap = right;
		}
		// This is special because areas of order O don't have a bitmap
		areas[O].write(FreeAreaList {
			next: None,
			buddies_state: &mut [],
		});

		// SAFETY: every element is properly initialized.
		let mut areas = unsafe { mem::MaybeUninit::array_assume_init(areas) };
		let mut end = start;

		// Allocate smaller areas until the alignment for O order areas is satisified.
		for i in 0..O {
			let area_size = 1 << i;
			if area_size > count {
				break;
			}
			*areas[i].buddies_state.first_mut().unwrap() = 1;
			if end.as_ptr() as usize & (PAGE_SIZE << i) > 0 {
				end.cast::<FreeArea>()
					.as_ptr()
					.write(FreeArea { next: None });
				count -= area_size;
				areas[i].next = Some(end.cast());
				// Set the first bit to 1 to pretend its buddy is allocated.
				// SAFETY: the pointer won't overflow
				end = NonNull::new_unchecked(end.as_ptr().add(1 << i));
			}
		}

		// Create as many O order areas as possible
		let mut prev_area_next = &mut areas[O].next;
		let order_size = 1 << O;
		while count >= order_size {
			let mut area = end.cast();
			ptr::write(area.as_ptr(), FreeArea { next: None });
			*prev_area_next = Some(area);
			prev_area_next = &mut area.as_mut().next;
			count -= order_size;
			end = NonNull::new_unchecked(end.as_ptr().add(order_size));
		}

		// Split the remaining pages into appropriately sized areas
		for order in (0..=O).rev() {
			let order_size = 1 << order;
			if order_size <= count {
				let area = end.cast();
				ptr::write(
					area.as_ptr(),
					FreeArea {
						next: areas[order].next,
					},
				);
				areas[order].next = Some(area);
				count -= order_size;
				end = NonNull::new_unchecked(end.as_ptr().add(order_size));
			}
		}

		Self {
			free_areas: areas,
			start,
			end,
			#[cfg(debug_assertions)]
			last_free_addr: None,
		}
	}

	/// Attempts to allocate an area of the given order.
	#[must_use]
	pub fn allocate(&mut self, order: u8) -> Result<Area, AllocateError> {
		if usize::from(order) > O {
			// Make the caller aware that its request will never succeed.
			return Err(AllocateError::OrderTooLarge);
		}

		for o in usize::from(order)..=O {
			if let Some(mut area) = self.free_areas[o].next {
				// SAFETY: The area is properly initialized as a FreeArea.
				self.free_areas[o].next = unsafe { area.as_ref().next };
				// Mark the left area as allocated.
				if usize::from(order) < O {
					// SAFETY: The area is in range
					unsafe { self.bitmap_toggle(area.cast(), o) };
				}

				// Split the area, then return the last chunk.
				for o in (usize::from(order)..o).rev() {
					// SAFETY: The pointer is valid.
					unsafe {
						area.as_ptr().write(FreeArea {
							next: self.free_areas[o].next,
						});
					}

					self.free_areas[o].next = Some(area);
					// Mark the left area as allocated.
					// SAFETY: The area is in range
					unsafe { self.bitmap_toggle(area.cast(), o) };

					let offset = 1 << o;
					// SAFETY: The offset is in range and won't be null.
					area = unsafe {
						NonNull::new_unchecked(area.cast::<Page>().as_ptr().add(offset)).cast()
					};
					debug_assert_eq!(area.as_ptr().align_offset(PAGE_SIZE), 0);
				}

				// SAFETY: we own it.
				unsafe {
					return Ok(Area::new_unchecked(area.cast(), order));
				}
			}
		}

		Err(AllocateError::Empty)
	}

	/// Deallocates the area of the given order. This may be a subarea.
	///
	/// ## Safety
	///
	/// The memory in the area may not be used after this call.
	#[must_use]
	#[track_caller]
	pub unsafe fn deallocate(&mut self, area: Area) -> Result<(), DeallocateError> {
		if usize::from(area.order()) > O {
			// Make the caller aware that its request will never succeed.
			// In fact, if this error is returned something is very wrong at the callsite.
			return Err(DeallocateError::OrderTooLarge);
		}

		// Ensure a valid range has been passed, to prevent potential buffer overflow exploits.
		let area_start = area.start().as_ptr() as usize;
		let area_end = area.start().as_ptr().wrapping_add(1 << area.order()) as usize;
		let self_start = self.start.as_ptr() as usize;
		let self_end = self.end.as_ptr() as usize;
		if !(self_start <= area_start && area_start <= self_end) {}
		if !(self_start <= area_end && area_end <= self_end) {}
		if !(area_start < area_end) {}
		if !(area_start < area_end) || !(self_start <= area_start && area_end <= self_end) {
			return Err(DeallocateError::OutOfBounds);
		}

		#[cfg(debug_assertions)]
		{
			debug_assert_ne!(self.last_free_addr, Some(area.start()), "Double free");
			self.last_free_addr = Some(area.start());
		}

		let mut order = area.order().into();
		let mut area = area.start();

		// Clear the page
		for i in 0..1usize << order {
			area.as_ptr().add(i).as_mut().unwrap_unchecked().clear();
		}

		loop {
			if order == O || self.bitmap_toggle(area, order) {
				// There are no more areas to merge
				// The buddy is still allocated
				let area: NonNull<FreeArea> = area.cast();
				// SAFETY: the passed in value is in range and unused.
				area.as_ptr().write(FreeArea {
					next: self.free_areas[order].next,
				});
				self.free_areas[order].next = Some(area);
				break;
			} else {
				// Merge the two areas
				// Find the other area
				let other = self.buddy_other_area(area, order);
				let mut prev = &mut self.free_areas[order].next;
				debug_assert!(prev.is_some());
				let mut curr = prev.unwrap_unchecked();
				let other = loop {
					if curr == other.cast() {
						*prev = curr.as_mut().next;
						break curr;
					} else {
						prev = &mut curr.as_mut().next;
						debug_assert!(curr.as_mut().next.is_some(), "No matching buddy");
						curr = curr.as_mut().next.unwrap_unchecked();
					}
				};
				area = self.buddy_left_area(area, order);
				order += 1;
			}
		}

		Ok(())
	}

	/// Returns the offset to the bits in the bitmap for the given area, along with
	/// the corresponding mask.
	///
	/// ## Safety
	///
	/// The area is in the managed range of this manager.
	#[inline]
	#[track_caller]
	#[must_use]
	unsafe fn bitmap_index<'a>(&'a self, area: NonNull<Page>, order: usize) -> (usize, usize) {
		// > is intentional, as the areas with order O cannot have buddies.
		debug_assert!(O > order, "order is too large");

		let mask = !((PAGE_SIZE << order) - 1);
		let offset = area.as_ptr() as usize;
		let offset = offset & mask - (self.start.as_ptr() as usize) & mask;
		let offset = offset / PAGE_SIZE;
		let offset = offset >> (order + 1);

		let bit_offset = offset & USIZE_MASK;
		let usize_offset = offset / USIZE_BITS;

		(usize_offset, 1 << bit_offset)
	}

	/// Toggles the bit of the given area and returns whether it's on or off.
	///
	/// ## Safety
	///
	/// The area is in the managed range of this manager.
	#[inline]
	#[track_caller]
	#[must_use]
	unsafe fn bitmap_toggle(&mut self, area: NonNull<Page>, order: usize) -> bool {
		let (offset, mask) = self.bitmap_index(area, order);
		let map = &mut self.free_areas[order].buddies_state;
		debug_assert!(offset < map.len(), "usize_offset out of bounds");
		let addr = map.get_unchecked_mut(offset);
		*addr ^= mask;
		*addr & mask > 0
	}

	/// Returns whether the bit of the given area is on or off.
	///
	/// ## Safety
	///
	/// The area is in the managed range of this manager.
	#[inline]
	#[track_caller]
	#[must_use]
	unsafe fn bitmap_get(&self, area: NonNull<Page>, order: usize) -> bool {
		let (offset, mask) = self.bitmap_index(area, order);
		let map = &self.free_areas[order].buddies_state;
		debug_assert!(offset < map.len(), "usize_offset out of bounds");
		let addr = map.get_unchecked(offset);
		*addr & mask > 0
	}

	/// Returns the buddy area of another area.
	///
	/// ## Safety
	///
	/// The area is in the managed range of this manager.
	#[inline]
	#[must_use]
	unsafe fn buddy_other_area(&self, area: NonNull<Page>, order: usize) -> NonNull<Page> {
		// There is no offset_from with usize that doesn't cause UB, so do this instead.
		let distance = area.as_ptr() as usize;
		let mask = PAGE_SIZE << order;
		// If it's a left area, it'll turn into the right one
		// If it's a right area, it'll turn into the left one
		// XOR is magic :)
		let distance = distance ^ mask;
		NonNull::new_unchecked(distance as *mut _)
	}

	/// Returns the left area of a buddy.
	///
	/// ## Safety
	///
	/// The area is in the managed range of this manager.
	#[inline]
	#[must_use]
	unsafe fn buddy_left_area(&self, area: NonNull<Page>, order: usize) -> NonNull<Page> {
		// There is no offset_from with usize that doesn't cause UB, so do this instead.
		let distance = area.as_ptr() as usize;
		let mask = PAGE_SIZE << order;
		// If it's a left area, it'll turn into the right one
		// If it's a left area, it'll remain a left one
		// AND is also magic :)
		let distance = distance & !mask;
		NonNull::new_unchecked(distance as *mut _)
	}

	/// Returns `true` if the area is properly aligned. `false` otherwise
	#[inline]
	#[must_use]
	fn is_area_aligned(area: NonNull<Page>, order: usize) -> bool {
		let mask = (PAGE_SIZE << order) - 1;
		(area.as_ptr() as usize) & mask == 0
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::log;

	/// Address that should be unused.
	const START: usize = 0x8100_0000;

	/// Creates a new memory manager.
	fn new<const O: usize>(count: usize) -> BuddyAllocator<O>
	where
		[(); O + 1]: Sized,
	{
		let start = NonNull::new(START as *mut Page).unwrap();
		unsafe { BuddyAllocator::<O>::new(start, NonZeroUsize::new(count).unwrap()) }
	}

	/// Checks the amount of free areas of the given order.
	#[track_caller]
	fn check<const O: usize>(manager: &BuddyAllocator<O>, order: usize, expected: usize)
	where
		[(); O + 1]: Sized,
	{
		let mut count = 0;
		let mut start = manager.free_areas[order].next;
		while let Some(mut s) = start {
			count += 1;
			start = unsafe { s.as_mut().next };
			if start == Some(s) {
				panic!("Cyclic reference");
			}
		}
		if count != expected {
			log::debug_usize("order", order, 10);
			log::debug_usize("expected count", expected, 10);
			log::debug_usize("real count", count, 10);
			panic!("Area count doesn't match expected count");
		}
	}

	/// Checks if the given bit is toggled properly
	#[track_caller]
	fn check_bm<const O: usize>(
		manager: &BuddyAllocator<O>,
		area: NonNull<Page>,
		order: usize,
		expected: bool,
	) where
		[(); O + 1]: Sized,
	{
		unsafe {
			assert_eq!(
				manager.bitmap_get(area, order),
				expected,
				"Bitmap doesn't match"
			)
		}
	}

	test!(create_0() {
		let mm = new::<0>(10);
		// There are no bitmaps, so we can utilize the full range.
		check(&mm, 0, 10);
	});

	test!(create_1() {
		let mm = new::<1>(10);
		check(&mm, 0, 1);
		check(&mm, 1, 4);
	});

	test!(create_2() {
		// mF FF | FF FF | FF
		let mm = new::<2>(10);
		check(&mm, 0, 1);
		check(&mm, 1, 2);
		check(&mm, 2, 1);
	});

	test!(create_3_min() {
		// mF FF | FF FF | FF
		let mm = new::<3>(10);
		check(&mm, 0, 1);
		check(&mm, 1, 2);
		check(&mm, 2, 1);
		check(&mm, 3, 0);
	});

	test!(create_3() {
		// mF FF | FF FF | FF FF | FF FF
		let mm = new::<3>(16);
		check(&mm, 0, 1);
		check(&mm, 1, 1);
		check(&mm, 2, 1);
		check(&mm, 3, 1);
	});

	test!(create_0_alloc_1x0() {
		let mut mm = new::<0>(10);
		let a = mm.allocate(0).unwrap();
		// There is no bitmap, hence the full range is 10
		check(&mm, 0, 9);
	});

	test!(create_0_alloc_2x0() {
		let mut mm = new::<0>(10);
		let a = mm.allocate(0).unwrap();
		let b = mm.allocate(0).unwrap();
		// Ditto
		check(&mm, 0, 8);
	});

	test!(create_1_alloc_1x1() {
		// xF | FF FF | FF FF
		let mut mm = new::<1>(10);
		// xF | aa FF | FF FF
		let a = mm.allocate(1).unwrap();
		check(&mm, 0, 1);
		check(&mm, 1, 3);
		check_bm(&mm, a.start(), 0, false);
	});

	test!(create_1_alloc_1x0() {
		// xF | FF FF | FF FF
		let mut mm = new::<1>(10);
		// xa | FF FF | FF FF
		let a = mm.allocate(0).unwrap();
		check(&mm, 0, 0);
		check(&mm, 1, 4);
		// False since the first page should be allocated, which has no buddy
		// (1 page for bitmap, 9 for allocation -> 1 unpaired)
		check_bm(&mm, a.start(), 0, false);
	});

	test!(create_1_alloc_1x0_1x1() {
		// xF | FF FF | FF FF
		let mut mm = new::<1>(10);
		// xa | FF FF | FF FF
		let a = mm.allocate(0).unwrap();
		// xa | bb FF | FF FF
		let b = mm.allocate(1).unwrap();
		check(&mm, 0, 0);
		check(&mm, 1, 3);
		// False since the last page should be allocated, which has no buddy
		check_bm(&mm, a.start(), 0, false);
	});

	test!(create_1_alloc_1x1_1x0() {
		// xF | FF FF | FF FF
		let mut mm = new::<1>(10);
		// xF | aa FF | FF FF
		let a = mm.allocate(1).unwrap();
		// xb | FF FF | FF FF
		let b = mm.allocate(0).unwrap();
		check(&mm, 0, 0);
		check(&mm, 1, 3);
		// False since the last page should be allocated, which has no buddy
		check_bm(&mm, b.start(), 0, false);
	});

	test!(create_2_alloc_1x0_1x1_1x0_1x2() {
		// xF FF | FF FF | FF
		let mut mm = new::<2>(10);
		// xa FF | FF FF | FF
		let a = mm.allocate(0).unwrap();
		check_bm(&mm, a.start(), 0, false); // x | a
		check_bm(&mm, a.start(), 1, true); // xa | FF
		// xa bb | FF FF | FF  OR  xa FF | FF FF | bb
		let b = mm.allocate(1).unwrap();
		check_bm(&mm, a.start(), 0, false); // x | a
		check_bm(&mm, b.start(), 0, false); // b | b
		// xa bb | FF FF | cF  OR  xa cF | FF FF | bb
		let c = mm.allocate(0).unwrap();
		check_bm(&mm, a.start(), 0, false); // x | a
		check_bm(&mm, a.start(), 1, false); // xa | bb  OR  xa | cF
		check_bm(&mm, b.start(), 0, false); // b | b
		check_bm(&mm, c.start(), 0, true); // c | F
		// xa bb | dd dd | cF  OR  xa cF | dd dd | bb
		let d = mm.allocate(2).unwrap();
		check(&mm, 0, 1);
		check(&mm, 1, 0);
		check(&mm, 2, 0);
		check_bm(&mm, a.start(), 0, false); // x | a
		check_bm(&mm, a.start(), 1, false); // xa | bb  OR  xa | cF
		check_bm(&mm, b.start(), 0, false); // b | b
		check_bm(&mm, c.start(), 0, true); // c | F
		check_bm(&mm, d.start(), 1, false); // dd | dd
		check_bm(&mm, d.start(), 0, false); // d | d
	});

	test!(create_0_alloc_1x0_dealloc_1x0() {
		let mut mm = new::<0>(10);
		let a = mm.allocate(0).unwrap();
		unsafe { mm.deallocate(a).unwrap(); }
		// No bitmap
		check(&mm, 0, 10);
	});

	test!(create_0_alloc_2x0_dealloc_2x0() {
		let mut mm = new::<0>(10);
		let a = mm.allocate(0).unwrap();
		let b = mm.allocate(0).unwrap();
		unsafe { mm.deallocate(a).unwrap(); }
		unsafe { mm.deallocate(b).unwrap(); }
		// No bitmap
		check(&mm, 0, 10);
	});

	test!(create_1_alloc_1x1_dealloc_1x1() {
		let mut mm = new::<1>(10);
		let a = mm.allocate(1).unwrap();
		unsafe { mm.deallocate(a).unwrap(); }
		check(&mm, 0, 1);
		check(&mm, 1, 4);
		check_bm(&mm, a.start(), 0, false);
	});

	test!(create_1_alloc_1x1_dealloc_2x0() {
		let mut mm = new::<1>(10);
		let a2 = mm.allocate(1).unwrap();
		let (a, b) = a2.split().unwrap();
		unsafe { mm.deallocate(a).unwrap(); }
		unsafe { mm.deallocate(b).unwrap(); }
		check(&mm, 0, 1);
		check(&mm, 1, 4);
		check_bm(&mm, a.start(), 0, false);
		check_bm(&mm, b.start(), 0, false);
	});

	test!(create_3_alloc_1x3_dealloc_2x1_1x1_2x0() {
		let mut mm = new::<3>(16);
		// Allocate the 7 first pages to make it easier to test what we actually want to test.
		// This means there are 8 pages left, grouped as an area of order 3.
		mm.allocate(0);
		mm.allocate(1);
		mm.allocate(2);
		let a4 = mm.allocate(3).unwrap();
		let (a2, b2) = a4.split().unwrap();
		let (a, b) = a2.split().unwrap();
		let (c, d) = b2.split().unwrap();
		// Intentionally fragment the memory ( FF xx | xx FF )
		unsafe { mm.deallocate(b).unwrap(); }
		unsafe { mm.deallocate(c).unwrap(); }
		check(&mm, 0, 0);
		check(&mm, 1, 2);
		check(&mm, 2, 0);
		check(&mm, 3, 0);
		check_bm(&mm, a.start(), 1, true);
		check_bm(&mm, b.start(), 1, true);
		check_bm(&mm, c.start(), 1, true);
		check_bm(&mm, d.start(), 1, true);
		// FF FF | xx FF
		unsafe { mm.deallocate(a).unwrap(); }
		check(&mm, 0, 0);
		check(&mm, 1, 1);
		check(&mm, 2, 1);
		check(&mm, 3, 0);
		check_bm(&mm, a.start(), 1, false);
		check_bm(&mm, b.start(), 1, false);
		check_bm(&mm, c.start(), 1, true);
		check_bm(&mm, d.start(), 1, true);
		// FF FF | FF FF
		let (e, f) = d.split().unwrap();
		unsafe { mm.deallocate(e).unwrap(); }
		unsafe { mm.deallocate(f).unwrap(); }
		check(&mm, 0, 0);
		check(&mm, 1, 0);
		check(&mm, 2, 0);
		check(&mm, 3, 1);
		check_bm(&mm, a.start(), 1, false);
		check_bm(&mm, b.start(), 1, false);
		check_bm(&mm, c.start(), 1, false);
		check_bm(&mm, d.start(), 1, false);
	});

	test!(err_alloc_empty() {
		let mut mm = new::<0>(2);
		let _ = mm.allocate(0).unwrap();
		let _ = mm.allocate(0).unwrap();
		assert_eq!(mm.allocate(0).unwrap_err(), AllocateError::Empty);
	});

	test!(err_alloc_order_too_large() {
		let mut mm = new::<1>(10);
		assert_eq!(mm.allocate(2).unwrap_err(), AllocateError::OrderTooLarge);
	});

	test!(err_dealloc_order_of_bounds() {
		let mut mm = new::<0>(4);
		let a = mm.allocate(0).unwrap().start();
		let e = unsafe { mm.deallocate(Area::new(NonNull::new(a.as_ptr().wrapping_sub(1)).unwrap(), 0).unwrap()) };
		assert_eq!(e.unwrap_err(), DeallocateError::OutOfBounds);
		let e = unsafe { mm.deallocate(Area::new(NonNull::new(a.as_ptr().wrapping_add(4)).unwrap(), 0).unwrap()) };
		assert_eq!(e.unwrap_err(), DeallocateError::OutOfBounds);
	});

	test!(err_dealloc_order_too_large() {
		let mut mm = new::<1>(10);
		let a = mm.allocate(1).unwrap();
		let a = unsafe { Area::new_unchecked(a.start(), 2) };
		assert_eq!(unsafe { mm.deallocate(a) }, Err(DeallocateError::OrderTooLarge));
	});
}
