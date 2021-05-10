use core::alloc::{AllocError, Allocator, Layout};
use core::cell::Cell;
use core::mem;
use core::ptr::NonNull;

/// A very simple memory allocator based on
/// https://wiki.osdev.org/Memory_Allocation#A_very_very_simple_Memory_Manager
///
/// This allocator is useful if you need to pack memory very densely or there
/// is no room for a more complicated memory manager.
///
/// Normally, this allocator should not be used as it is very slow and doesn't account for
/// fragmentation.
///
/// ## How it works, briefly
///
/// Adapted from the OSDev wiki:
///
/// > ... one of the easiest solution is to put at the start of the freed zone a descriptor that
/// > allows you to insert it in a list of free zones. Keeping that list sorted by address helps
/// > you identifying contiguous free zones and allows you to merge them in larger free zones.
/// >
/// > ```
/// >       first free                     Structure of a   +--.--.--.--+    +---> +-- ...
/// >        \/                            free zone        |F  R  E  E |    |     | FREE
/// > +----+-----+--+----+---+-----//--+                    |nextfreeptr|----+
/// > |##A#|free |C#|free|#E#|   free  |               <----|           |
/// > +----+-|---+--+-|--+---+-----//--+                    | zone size |
/// > +-next>--+                                            +///////////+
/// > ```
///
/// Caveat is that every zone has to be at least `2 * size_of::<usize>()` bytes large. We also
/// don't keep track of a `prevfreeptr` to keep the implementation simple.
///
/// ## Notes
///
/// Trying to allocate memory with `layout.align() > 2 * size_of::<usize>()` will return
/// `AllocError`. This is too keep the implementation simple.
/// Generally, the only time alignment matters is when working with vector instructions (e.g. SSE
/// or AVX). If your system supports such instructions and you need them, then use a different
/// allocator.
pub struct WaterMark {
	/// The base address of the allocator
	base: NonNull<u8>,
	/// The lowest (i.e. first) free address pointing to a free zone. May be `None` if the heap is
	/// full.
	next: Cell<Option<NonNull<FreeZone>>>,
}

#[repr(C)]
struct FreeZone {
	/// The size of this zone. Must be a multiple of `size_of::<Self>()`
	size: usize,
	/// A pointer to the next zone. May be `None` if the heap is full.
	next: Option<NonNull<Self>>,
}

const MIN_SIZE: usize = mem::size_of::<FreeZone>();

impl WaterMark {
	/// Creates a new `WaterMark` allocator.
	///
	/// ## Safety
	///
	/// `base` is valid and doesn't point to memory used by other structures
	pub unsafe fn new(base: NonNull<u8>, size: usize) -> Self {
		let zone: &mut FreeZone = base.cast().as_mut();
		zone.size = size;
		zone.next = None;
		Self {
			base,
			next: Cell::new(Some(NonNull::from(zone))),
		}
	}
}

unsafe impl Allocator for WaterMark {
	fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		// Don't attempt to allocate memory if we need to ensure proper alignment
		// This is to keep the implementation simple.
		if layout.align() > MIN_SIZE {
			return Err(AllocError);
		}

		// If it's zero-sized, then just return an aligned pointer
		if layout.size() == 0 {
			let ptr = NonNull::new(layout.align() as *mut u8).unwrap();
			return Ok(NonNull::slice_from_raw_parts(ptr, 0));
		}

		// Ensure the zone can be split into chunks of FreeZones
		const _POW2_ASSERT: usize = 0 - (MIN_SIZE & (MIN_SIZE - 1));
		let min_size = (layout.size() + MIN_SIZE - 1) & !(layout.size() - 1);

		let mut prev_zone: Option<&mut FreeZone> = None;
		let mut maybe_current = self.next.get();

		while let Some(mut curr_zone_ptr) = maybe_current {
			// SAFETY: The pointer is valid
			let curr_zone = unsafe { curr_zone_ptr.as_mut() };
			if curr_zone.size == min_size {
				// Fix the next of the previous zone
				if let Some(prev_zone) = prev_zone {
					prev_zone.next = curr_zone.next;
				} else {
					self.next.set(curr_zone.next);
				}
				let ptr = curr_zone_ptr.cast();
				return Ok(NonNull::slice_from_raw_parts(ptr, layout.size()));
			} else if curr_zone.size > min_size {
				// Simply resize the current zone and offset the allocated memory.
				curr_zone.size -= min_size;
				// SAFETY: the pointer won't overflow & the pointed memory is unused
				let ptr = unsafe {
					let ptr = curr_zone_ptr.as_ptr().offset(curr_zone.size as isize);
					NonNull::new_unchecked(ptr).cast()
				};
				return Ok(NonNull::slice_from_raw_parts(ptr, layout.size()));
			}
			maybe_current = curr_zone.next;
			prev_zone = Some(curr_zone);
		}

		Err(AllocError)
	}

	unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
		todo!()
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::log;

	// TODO figure out a way to find unused memory
	// Right now, we're just praying this doesn't land in either the stack or the
	// executable/.rodata/.data/... sections.
	const UNUSED_MEM_PTR: NonNull<u8> = unsafe { NonNull::new_unchecked(0x8100_0000 as *mut u8) };

	test!(allocate_byte() {
		let allocator = unsafe { WaterMark::new(UNUSED_MEM_PTR, 128) };
		let layout = Layout::from_size_align(1, 1).unwrap();
		let _ = allocator.allocate_zeroed(layout);
	});

	test!(allocate_zst() {
		let allocator = unsafe { WaterMark::new(UNUSED_MEM_PTR, 128) };
		let layout = Layout::from_size_align(0, 1).unwrap();
		let data = allocator.allocate_zeroed(layout).unwrap();
		let data = unsafe { data.as_ref() };
		assert_eq!(data.len(), 0);
		assert_eq!(data.as_ptr_range().start, 0x1 as *mut u8);
	});
}
