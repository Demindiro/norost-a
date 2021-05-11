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
		let min_size = (layout.size() + MIN_SIZE - 1) & !(MIN_SIZE - 1);

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
				super::track_allocation(ptr, min_size);
				return Ok(NonNull::slice_from_raw_parts(ptr, layout.size()));
			} else if curr_zone.size > min_size {
				// Simply resize the current zone and offset the allocated memory.
				curr_zone.size -= min_size;
				// SAFETY: the pointer won't overflow & the pointed memory is unused
				let ptr = unsafe {
					let ptr = curr_zone_ptr.cast::<u8>().as_ptr().add(curr_zone.size);
					NonNull::new_unchecked(ptr).cast()
				};
				super::track_allocation(ptr, min_size);
				return Ok(NonNull::slice_from_raw_parts(ptr, layout.size()));
			}
			maybe_current = curr_zone.next;
			prev_zone = Some(curr_zone);
		}

		Err(AllocError)
	}

	unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
		let min_size = (layout.size() + MIN_SIZE - 1) & !(MIN_SIZE - 1);
		super::track_deallocation(ptr, min_size);
		if let Some(mut zone) = self.next.get() {
			if (zone.as_ptr() as usize) < ptr.as_ptr() as usize {
				// Find the free zone right before ptr
				while let Some(zn) = zone.as_mut().next {
					if (zn.as_ptr() as usize) < ptr.as_ptr() as usize {
						zone = zn;
					} else {
						break;
					}
				}

				// Find the free zone right after ptr, which is simply the zone right after
				let next_zone = zone.as_mut().next;

				// Check if the left and middle (allocated) zone can be merged
				let mut zone = if zone.as_ptr().cast::<u8>().add(zone.as_mut().size) == ptr.as_ptr()
				{
					zone.as_mut().size += min_size;
					zone
				} else {
					let nz = FreeZone {
						size: min_size,
						next: next_zone,
					};
					let ptr = ptr.cast::<FreeZone>();
					ptr.as_ptr().write(nz);
					zone.as_mut().next = Some(ptr);
					ptr.cast()
				};

				// Check if the middle and right zone can be merged
				if let Some(mut next_zone) = next_zone {
					if zone.as_ptr().cast::<u8>().add(zone.as_mut().size)
						== next_zone.cast().as_ptr()
					{
						zone.as_mut().size += next_zone.as_mut().size;
						zone.as_mut().next = next_zone.as_mut().next;
					}
				}
			} else {
				// Write a new zone, as there is no possibility for a merge
				let nz = FreeZone {
					size: min_size,
					next: Some(zone),
				};
				let mut ptr = ptr.cast::<FreeZone>();
				ptr.as_ptr().write(nz);
				zone.as_mut().next = Some(ptr);
				// Check if the middle (allocated) and right zone can be merged
				if ptr.cast::<u8>().as_ptr().add(min_size) == zone.cast().as_ptr() {
					ptr.as_mut().size += zone.as_mut().size;
					ptr.as_mut().next = zone.as_mut().next;
				}
				// The new zone is first, so set self.next appropriately
				self.next.set(Some(ptr.cast()));
			}
		} else {
			// All zones are occupied, so just write out
			let zone = FreeZone {
				size: layout.size(),
				next: None,
			};
			ptr.cast::<FreeZone>().as_ptr().write(zone);
			self.next.set(Some(ptr.cast()));
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::alloc;
	use crate::log;

	/// Since we're just running tests, we'll assume that anything at this address here is unused.
	/// Adjust as needed.
	// TODO Maybe it's worth adding a linker symbol for this instead?
	const UNUSED_MEM_PTR: NonNull<u8> = unsafe { NonNull::new_unchecked(0x8100_0000 as *mut u8) };
	// Ditto?
	/// Presumably there is at least 16KB memory free on whatever platform the kernel can run on.
	const UNUSED_MEM_SIZE: usize = 0x4_000;

	/// Allocate a temporary heap for testing.
	fn make_heap() -> WaterMark {
		unsafe { WaterMark::new(UNUSED_MEM_PTR, UNUSED_MEM_SIZE) }
	}

	test!(allocate_byte() {
		let allocator = make_heap();
		let layout = Layout::from_size_align(1, 1).unwrap();
		let _ = allocator.allocate_zeroed(layout);
	});

	test!(allocate_zst() {
		let allocator = make_heap();
		let layout = Layout::from_size_align(0, 1).unwrap();
		let data = allocator.allocate_zeroed(layout).unwrap();
		let data = unsafe { data.as_ref() };
		assert_eq!(data.len(), 0);
		assert_eq!(data.as_ptr_range().start, 0x1 as *mut u8);
	});

	test!(allocate_box_3() {
		let heap = make_heap();
		for i in 0..3 {
			alloc::Box::try_new_in(0, &heap).expect("Failed to allocate");
		}
	});

	test!(allocate_box_3_chain() {
		let heap = make_heap();
		let _a = alloc::Box::try_new_in(0, &heap).expect("Failed to allocate");
		let _b = alloc::Box::try_new_in(0, &heap).expect("Failed to allocate");
		let _c = alloc::Box::try_new_in([0u8; 100], &heap).expect("Failed to allocate");
	});

	test!(allocate_box_3_chain_drop_mid_first() {
		let heap = make_heap();
		let _a = alloc::Box::try_new_in(0, &heap).expect("Failed to allocate");
		let _b = alloc::Box::try_new_in(0, &heap).expect("Failed to allocate");
		let _c = alloc::Box::try_new_in([0u8; 100], &heap).expect("Failed to allocate");
		drop(_b);
	});

	test!(allocate_boxslice() {
		let heap = make_heap();
		let a = alloc::Box::<[usize], _>::try_new_uninit_slice_in(40, &heap).expect("Failed to allocate");
		assert_eq!(a.len(), 40);
	});
}
