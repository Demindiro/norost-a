//! # Address range reservations & IPC queues.
//!
//! As the kernel does not have any form of CoW, it is impractical to allocate a large range
//! of pages upfront. However, simply leaving the gaps open and allocating anywhere may
//! interfere with some systems such as the memory allocator.
//!
//! To address this, *page reservations* are tracked in a single place. If a need for a contiguous
//! range of pages is anticipated, it can be reserved & tracked here.
//!
//! It also manages

use core::mem;
use core::ptr::NonNull;
use core::slice;
use core::sync::atomic::{AtomicPtr, AtomicU16, AtomicUsize, Ordering};

/// The end address of the null page. The null page can never be allocated.
const NULL_PAGE_END: NonNull<()> =
	unsafe { NonNull::new_unchecked((kernel::Page::SIZE - 1) as *mut _) };

/// A single memory range.
#[repr(C)]
struct MemoryMap {
	start: AtomicPtr<kernel::Page>,
	/// end is *inclusive*, i.e. the offset bits are all `1`s.
	end: AtomicPtr<()>,
}

/// A sorted list of reserved memory ranges.
#[repr(C)]
struct MemoryMapList {}

struct IPCQueueIndex {
	mask: u16,
	index: u16,
}

/// An IPC queue
#[repr(C)]
struct IPCQueue {}

/// Part of the global structure but separate to account for be able to account for memory size.
#[repr(C)]
struct GlobalPart {
	transmit_mask: AtomicU16,
	transmit_index: AtomicU16,
	receive_mask: AtomicU16,
	receive_index: AtomicU16,
	transmit_entries: AtomicPtr<kernel::ipc::Packet>,
	receive_entries: AtomicPtr<kernel::ipc::Packet>,
	/// The amount of occupied reserved entries
	reserved_count: AtomicUsize,
	/// The amount of available reserved entries.
	reserved_capacity: AtomicUsize,
	/// A pointer to extra reserved entries.
	extra_reserved_entries: AtomicPtr<MemoryMap>,
	/// The amount of occupied reserved entries.
	free_ranges_count: AtomicUsize,
	/// The amount of available reserved entries.
	///
	/// If it is 0, the range is locked (TODO).
	free_ranges_capacity: AtomicUsize,
	/// A list of free ranges for use with IPC.
	free_ranges: AtomicPtr<kernel::ipc::FreePage>,
}

/// The whole memory management structure.
///
/// One such structure is located at a fixed address. This is done this way so there is no reliance
/// on library loaders, linkers ... to set the correct address which should in turn lead to less
/// potential surprises.
///
/// The elements are arranged such that the packing is as optimal as possible for all architectures.
#[repr(C)]
struct Global {
	part: GlobalPart,
	/// "inline" reserved entries which are stored directly in the structure to make optimal use of
	/// the remaining bytes in the page.
	reserved_entries: [MemoryMap;
		(kernel::Page::SIZE - mem::size_of::<GlobalPart>()) / mem::size_of::<MemoryMap>()],
}

/// Ensure the `Global` structure has an optimal size.
const _: usize = mem::size_of::<usize>() - (kernel::Page::SIZE - mem::size_of::<Global>());

// TODO using `const` gives "unable to turn bytes into a pointer".
//
// I understand why the compiler would consider this UB, but AEEEEEUUUURGH *dies*.
const GLOBAL_PTR: *mut Global = (0x0fff_ffff_usize & !(kernel::Page::SIZE - 1)) as *mut _;

// Lol, lmao, this won't backfire, pinky promise.
const GLOBAL: Magic = Magic;

struct Magic;

impl core::ops::Deref for Magic {
	type Target = Global;

	fn deref(&self) -> &Global {
		// You didn't see anything, carry on fellow reader
		unsafe { &*GLOBAL_PTR }
	}
}

/// Initializes the library. This should be the first function called in crt0.
// TODO this should be moved to a separate library inside `rtbegin.rs` or even just `¢rt0.rs`
pub unsafe fn init() {
	// FIXME need a mem_get_mappings syscall of sorts.

	// Allocate a page for the global struct.
	let ret = kernel::mem_alloc(GLOBAL_PTR.cast(), 1, kernel::PROT_READ_WRITE);
	if (ret.status != 0) {
		// FIXME handle errors properly
		todo!()
	}
	// kernel_mem_alloc only returns zeroed pages, so accessing the struct is safe.
	GLOBAL
		.part
		.reserved_capacity
		.store(GLOBAL.reserved_entries.len(), Ordering::Relaxed);

	// Immediately register the global page itself and reserve some pages for it.
	GLOBAL.reserved_entries[1]
		.start
		.store(GLOBAL_PTR.cast(), Ordering::Relaxed);
	GLOBAL.reserved_entries[1].end.store(
		GLOBAL_PTR.cast::<u8>().add(super::Page::SIZE - 1).cast(),
		Ordering::Relaxed,
	);
	//reserved_count = 1;

	// FIXME assume the top and bottom are reserved for stack and ELF respectively.
	GLOBAL.reserved_entries[2]
		.start
		.store(0xfff00000 as *mut _, Ordering::Relaxed);
	GLOBAL.reserved_entries[2]
		.end
		.store(0xfffeffff as *mut _, Ordering::Relaxed);
	GLOBAL.reserved_entries[0]
		.start
		.store(0x10000 as *mut _, Ordering::Relaxed);
	GLOBAL.reserved_entries[0]
		.end
		.store(0x1ffffff as *mut _, Ordering::Relaxed);
	GLOBAL.part.reserved_count.store(3, Ordering::Relaxed);

	// Reserve pages for IPC
	// FIXME handle errors properly
	let addr = reserve_range(None, 8).unwrap();
	let ret = kernel::mem_alloc(addr.as_ptr(), 1, kernel::PROT_READ_WRITE);
	if (ret.status != 0) {
		// FIXME handle errors properly
		todo!()
	}

	let addr = addr.as_ptr().cast::<kernel::ipc::Packet>();

	let txq = addr;
	GLOBAL.part.transmit_entries.store(txq, Ordering::Relaxed);
	GLOBAL.part.transmit_mask.store(0, Ordering::Relaxed);
	GLOBAL.part.transmit_index.store(0, Ordering::Relaxed);

	let rxq = addr.add(1);
	GLOBAL.part.receive_entries.store(rxq, Ordering::Relaxed);
	GLOBAL.part.receive_mask.store(0, Ordering::Relaxed);
	GLOBAL.part.receive_index.store(0, Ordering::Relaxed);

	let free_ranges = addr.add(2).cast();
	GLOBAL
		.part
		.free_ranges
		.store(free_ranges, Ordering::Relaxed);
	GLOBAL.part.free_ranges_capacity.store(1, Ordering::Relaxed);

	// Set a range to which pages can be mapped to.
	let free_ranges = unsafe { slice::from_raw_parts_mut(free_ranges, 1) };
	// FIXME Ditto
	let addr = reserve_range(None, 8).unwrap();
	add_free_range(addr, 1).unwrap();

	// Register the queues to the kernel
	let ret = kernel::io_set_queues(
		txq,
		0,
		rxq,
		0,
		free_ranges.as_ptr() as *mut _,
		free_ranges.len(),
	);
	if (ret.status != 0) {
		// FIXME handle errors properly
		todo!()
	}
}

/// Insert a memory reservation entry. The index must be lower than reserved_count.
fn mem_insert_entry(
	index: usize,
	start: NonNull<kernel::Page>,
	end: NonNull<()>,
) -> Result<(), ()> {
	let count = GLOBAL.part.reserved_count.fetch_add(1, Ordering::Relaxed);
	let capacity = GLOBAL.part.reserved_capacity.load(Ordering::Relaxed);
	if count >= capacity {
		// TODO allocate additional pages if needed.
		return Err(());
	}
	// Shift all entries at and after the index up.
	let entries = &GLOBAL.reserved_entries;
	for i in (index + 1..=count).rev() {
		let start = GLOBAL.reserved_entries[i - 1].start.load(Ordering::Relaxed);
		let end = GLOBAL.reserved_entries[i - 1].end.load(Ordering::Relaxed);
		GLOBAL.reserved_entries[i]
			.start
			.store(start, Ordering::Relaxed);
		GLOBAL.reserved_entries[i].end.store(end, Ordering::Relaxed);
	}
	// Write the entry.
	GLOBAL.reserved_entries[index]
		.start
		.store(start.as_ptr(), Ordering::Relaxed);
	GLOBAL.reserved_entries[index]
		.end
		.store(end.as_ptr(), Ordering::Relaxed);
	Ok(())
}

#[derive(Debug)]
pub enum ReserveError {
	/// Failed to allocate memory
	NoMemory,
	/// There is no free range large enough
	NoSpace,
}

pub fn reserve_range(
	address: Option<super::Page>,
	count: usize,
) -> Result<super::Page, ReserveError> {
	if let Some(address) = address {
		// Do a binary search, check if there is enough space & insert if so.
		todo!()
	} else {
		// Find the first range with enough space.
		// TODO maybe it's better if we try to find the tightest space possible? Or maybe
		// the widest space instead?
		let mut prev_end = NULL_PAGE_END.as_ptr().cast::<u8>();
		let reserved_count = GLOBAL.part.reserved_count.load(Ordering::Relaxed);
		for i in 0..reserved_count {
			let mm = &GLOBAL.reserved_entries[i];
			let start = prev_end.wrapping_add(1);
			let end = start.wrapping_add(count * super::Page::SIZE - 1);
			if (prev_end as usize) < start as usize
				&& (end as usize) < mm.start.load(Ordering::Relaxed) as usize
			{
				// There is enough space, so use it.
				match mem_insert_entry(
					i,
					NonNull::new(start).unwrap().cast(),
					NonNull::new(end).unwrap().cast(),
				) {
					Err(()) => return Err(ReserveError::NoMemory),
					Ok(()) => return Ok(unsafe { super::Page::new_unchecked(start.cast()) }),
				}
			}
			prev_end = mm.end.load(Ordering::Relaxed).cast();
		}
		return Err(ReserveError::NoSpace);
	}
}

#[derive(Debug)]
pub enum UnreserveError {
	/// There is no entry with the given address.
	InvalidAddress,
	/// The size of the entry is too large.
	SizeTooLarge,
}

pub fn unreserve_range(address: super::Page, count: usize) -> Result<(), UnreserveError> {
	GLOBAL.reserved_entries[..GLOBAL.part.reserved_count.load(Ordering::Relaxed)]
		.binary_search_by(|e| {
			((e.start.load(Ordering::Relaxed)) as usize).cmp(&(address.as_ptr() as usize))
		})
		// TODO check for size
		.map(|i| {
			GLOBAL.reserved_entries[i]
				.start
				.store(core::ptr::null_mut(), Ordering::Relaxed)
		})
		.map_err(|_| UnreserveError::InvalidAddress)
}

/* TODO how should we implement this safely?
struct kernel_ipc_packet *dux_reserve_transmit_entry(void) -> {
	return txq;
}

struct kernel_ipc_packet *dux_get_receive_entry(void) {
	return rxq;
}
*/

pub fn add_free_range(page: super::Page, count: usize) -> Result<(), ()> {
	// FIXME
	unsafe {
		(&mut *GLOBAL.part.free_ranges.load(Ordering::Relaxed)).address =
			Some(page.as_non_null_ptr());
		(&mut *GLOBAL.part.free_ranges.load(Ordering::Relaxed)).count = count;
	}
	Ok(())
}
