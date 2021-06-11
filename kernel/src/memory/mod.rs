//! Global memory manager
//!
//! This module keeps track of free pages in a given range of memory. It also manages access to
//! MMIO and DMA pages.
//!
//! Regular pages (i.e. non-MMIO/DMA pages) are reference counted with a binary tree.

pub use crate::arch::{Page, PAGE_SIZE};
use core::cell::UnsafeCell;
use core::num::NonZeroUsize;
use core::ptr::{self, NonNull};
use core::{mem, slice};

pub mod reserved;

mod allocator;
mod ppn;
mod shared;

pub use shared::SharedPPN;

use crate::sync::Mutex;

use allocator::Allocator;
use ppn::PPNBox;

pub use ppn::{PPN, PPNRange};

#[derive(Debug)]
pub struct AllocateError;

/// The global memory allocator.
///
/// The maximum area order varies for each architecture depending on hugepage support and practical
/// memory sizes. It is configured through the `MEMORY_ORDER` environment variable.
///
/// TODO SNIP THE BELOW
///
/// This is set to 27 which allows
/// areas up to 512 GiB, or a single "terapage" in RISC-V lingo. This should be
/// sufficient for a very, very long time.
///
/// See [`memory`](crate::memory) for more information.
///
/// ## References
///
/// Mention of "terapages" can be found in [the RISC-V manual for privileged instructions][riscv],
/// "Sv48: Page-Based 48-bit Virtual-Memory System", section 4.5.1, page 37.
///
/// [riscv]: https://github.com/riscv/riscv-isa-manual/releases/download/Ratified-IMFDQC-and-Priv-v1.11/riscv-privileged-20190608.pdf
static mut ALLOCATOR: Option<Mutex<allocator::Allocator>> = None;

/// Add a memory range for management. Currently only one range is supported.
///
/// ## Safety
///
/// The memory range must not be in use by anything else. It must also behave
/// like "regular" memory (i.e. not MMIO or non-existent)
///
/// ## Panics
///
/// It will panic if more than one memory range is registered. It will also panic if an unaligned
/// page is passed.
#[cold]
#[optimize(size)]
pub unsafe fn mem_add_ranges(ranges: &mut [PPNRange]) {
	#[cfg(not(test))]
	if ALLOCATOR.is_some() {
		panic!("Can't add more than one memory range");
	}
	ALLOCATOR = Some(Mutex::new(Allocator::new(ranges).unwrap()));
}

/// Allocate an area, i.e. a range of pages. This area is aligned such that all bits below
/// `PAGE_SIZE << order`` are zero.
#[optimize(speed)]
pub fn mem_allocate(order: u8) -> Result<PPN, AllocateError> {
	#[cfg(debug_assertions)]
	unsafe {
		Ok(ALLOCATOR.as_ref().expect("No initialized buddy allocator").lock().alloc().unwrap())
	}
	#[cfg(not(debug_assertions))]
	unsafe {
		Ok(ALLOCATOR.as_ref().unwrap_unchecked().lock().alloc().unwrap())
	}
}

pub fn allocate() -> Result<PPN, AllocateError> {
	mem_allocate(0)
}

/// Allocate a number of pages. The pages are not necessarily contiguous. To avoid needing to
/// lock once per page returned or needing an array to write out to, a closure must be passed
/// instead which can write the allocated pages out directly to whatever structure.
///
/// This may fail even when some pages are allocated. It is up to the caller to deallocate them.
#[optimize(speed)]
pub fn mem_allocate_range<F>(count: usize, mut f: F) -> Result<(), ()>
where
	F: FnMut(PPN),
{
	for _ in 0..count {
		// TODO add a re-entrant lock to workaround this sillyness.
		#[cfg(debug_assertions)]
		let mut a = unsafe { ALLOCATOR.as_ref().expect("No initialized buddy allocator").lock() };
		#[cfg(not(debug_assertions))]
		let mut a = unsafe { ALLOCATOR.as_ref().unwrap_unchecked().lock() };
		let p = a.alloc()?;
		drop(a);
		f(p);
	}
	Ok(())
}

/// Dereference an area, i.e. a range of pages. If the reference count reaches zero, the area is
/// zeroed out and deallocated.
#[optimize(speed)]
pub fn mem_deallocate(page: PPN) {
	#[cfg(debug_assertions)]
	unsafe {
		ALLOCATOR.as_ref().expect("No initialized buddy allocator").lock().free(page)
	}
	#[cfg(not(debug_assertions))]
	unsafe {
		ALLOCATOR.as_ref().unwrap_unchecked().lock().free(page)
	}
}
