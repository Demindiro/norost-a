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

use crate::util;
use crate::Page;
use core::cell::Cell;
use core::mem;
use core::ptr;
use core::ptr::NonNull;
use core::slice;
use core::sync::atomic;
use core::sync::atomic::{AtomicU16, AtomicUsize, Ordering};

/// A single memory range.
#[repr(C)]
struct MemoryMap {
	start: Cell<Option<Page>>,
	/// end is *inclusive*, i.e. the offset bits are all `1`s.
	end: Cell<*mut kernel::Page>,
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
	/// The mask of the transmit queue, which is the length of the queue `- 1`.
	///
	/// If it is `2`, the queue is locked.
	transmit_mask: AtomicU16,
	transmit_index: Cell<u16>,
	/// The mask of the receive queue, which is the length of the queue `- 1`.
	///
	/// If it is `2`, the queue is locked.
	receive_mask: AtomicU16,
	receive_index: Cell<u16>,
	transmit_entries: Cell<*mut kernel::ipc::Packet>,
	receive_entries: Cell<*mut kernel::ipc::Packet>,
	/// The amount of occupied reserved entries
	reserved_count: Cell<usize>,
	/// The amount of available reserved entries.
	///
	/// If this is 0, the list is locked.
	reserved_capacity: AtomicUsize,
	/// A pointer to extra reserved entries.
	extra_reserved_entries: Cell<*mut MemoryMap>,
	/// The amount of available free range entries.
	///
	/// If it is 0, the list is locked.
	free_ranges_capacity: AtomicUsize,
	/// A list of free ranges for use with IPC.
	free_ranges: Cell<*mut kernel::ipc::FreePage>,
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
const GLOBAL_PTR: *mut Global = (0x0fff_ffff & !(kernel::Page::SIZE - 1)) as *mut _;

const GLOBAL_PAGE: Page = unsafe { Page::new_unchecked(GLOBAL_PTR.cast()) };

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
// TODO maybe this should be written in assembly? We can avoid using the stack if we do.
pub unsafe fn init() {
	// FIXME need a mem_get_mappings syscall of sorts.

	// Allocate a page for the global struct.
	let ret = kernel::mem_alloc(GLOBAL_PTR.cast(), 1, kernel::PROT_READ_WRITE);
	if ret.status != 0 {
		// FIXME handle errors properly
		todo!()
	}
	// kernel_mem_alloc only returns zeroed pages, so accessing the struct is safe.
	GLOBAL
		.part
		.reserved_capacity
		.store(GLOBAL.reserved_entries.len(), Ordering::Relaxed);

	// Immediately register the global page itself and reserve some pages for it.
	GLOBAL.reserved_entries[1].start.set(Some(GLOBAL_PAGE));
	GLOBAL.reserved_entries[1]
		.end
		.set(GLOBAL_PTR.cast::<u8>().add(Page::SIZE - 1).cast());
	//reserved_count = 1;

	// FIXME assume the top and bottom are reserved for stack and ELF respectively.
	GLOBAL.reserved_entries[2]
		.start
		.set(Some(crate::Page::new_unchecked(0xfff00000 as *mut _)));
	GLOBAL.reserved_entries[2].end.set(0xfffeffff as *mut _);
	GLOBAL.reserved_entries[0]
		.start
		.set(Some(crate::Page::new_unchecked(0x10000 as *mut _)));
	GLOBAL.reserved_entries[0].end.set(0x1ffffff as *mut _);
	GLOBAL.part.reserved_count.set(3);

	// Reserve pages for IPC
	// FIXME handle errors properly
	let addr = reserve_range(None, 8).unwrap();
	let ret = kernel::mem_alloc(addr.as_ptr(), 1, kernel::PROT_READ_WRITE);
	if ret.status != 0 {
		// FIXME handle errors properly
		todo!()
	}

	let addr = addr.as_ptr().cast::<kernel::ipc::Packet>();

	let txq = addr;
	GLOBAL.part.transmit_entries.set(txq);
	GLOBAL.part.transmit_index.set(0);
	GLOBAL.part.transmit_mask.store(0, Ordering::Release);

	let rxq = addr.add(1);
	GLOBAL.part.receive_entries.set(rxq);
	GLOBAL.part.receive_index.set(0);
	GLOBAL.part.receive_mask.store(0, Ordering::Release);

	let free_ranges = addr.add(2).cast();
	let free_ranges_len = 12;
	GLOBAL.part.free_ranges.set(free_ranges);
	GLOBAL.part.free_ranges_capacity.store(free_ranges_len, Ordering::Release);

	// Set a range to which pages can be mapped to.
	let free_ranges = unsafe { slice::from_raw_parts_mut(free_ranges, free_ranges_len) };
	let count = free_ranges_len; // The effective maximum right now.
	let addr = reserve_range(None, count).unwrap();
	ipc::add_free_range(addr, count).unwrap();

	// Register the queues to the kernel
	let ret = kernel::io_set_queues(
		txq,
		0,
		rxq,
		0,
		free_ranges.as_ptr() as *mut _,
		free_ranges.len(),
	);
	if ret.status != 0 {
		// FIXME handle errors properly
		todo!()
	}
}

/// Insert a memory reservation entry. The index must be lower than reserved_count.
///
/// # Safety
///
/// The caller must have a lock on the reserved list.
unsafe fn mem_insert_entry(
	index: usize,
	start: crate::Page,
	end: NonNull<kernel::Page>,
	capacity: &mut usize,
) -> Result<(), ()> {
	let count = GLOBAL.part.reserved_count.get();
	if count >= *capacity {
		// TODO allocate additional pages if needed.
		return Err(());
	}
	GLOBAL.part.reserved_count.set(count + 1);
	// Shift all entries at and after the index up.
	let entries = &GLOBAL.reserved_entries;
	for i in (index + 1..=count).rev() {
		let start = GLOBAL.reserved_entries[i - 1].start.get();
		let end = GLOBAL.reserved_entries[i - 1].end.get();
		GLOBAL.reserved_entries[i].start.set(start);
		GLOBAL.reserved_entries[i].end.set(end);
	}
	// Write the entry.
	GLOBAL.reserved_entries[index].start.set(Some(start));
	GLOBAL.reserved_entries[index].end.set(end.as_ptr());
	Ok(())
}

#[derive(Debug)]
pub enum ReserveError {
	/// Failed to allocate memory
	NoMemory,
	/// There is no free range large enough
	NoSpace,
}

pub fn reserve_range(address: Option<Page>, count: usize) -> Result<Page, ReserveError> {
	util::spin_lock(&GLOBAL.part.reserved_capacity, 0, |capacity| {
		if let Some(address) = address {
			// Do a binary search, check if there is enough space & insert if so.
			todo!()
		} else {
			// Find the first range with enough space.
			// TODO maybe it's better if we try to find the tightest space possible? Or maybe
			// the widest space instead?
			let mut prev_end = Page::NULL_PAGE_END.cast::<u8>();
			let reserved_count = GLOBAL.part.reserved_count.get();
			for i in 0..reserved_count {
				let mm = &GLOBAL.reserved_entries[i];
				let start = prev_end.wrapping_add(1);
				let end = start.wrapping_add(count * Page::SIZE - 1);
				if prev_end < start
					&& end
						< mm.start
							.get()
							.map(|p| p.as_ptr().cast())
							.unwrap_or_else(ptr::null_mut)
				{
					// There is enough space, so use it.
					let start = unsafe { Page::new_unchecked(start.cast()) };
					let end = NonNull::new(end).unwrap().cast();
					match unsafe { mem_insert_entry(i, start, end, capacity) } {
						Err(()) => return Err(ReserveError::NoMemory),
						Ok(()) => return Ok(start),
					}
				}
				prev_end = mm.end.get().cast();
			}
			Err(ReserveError::NoSpace)
		}
	})
}

#[derive(Debug)]
pub enum UnreserveError {
	/// There is no entry with the given address.
	InvalidAddress,
	/// The size of the entry is too large.
	SizeTooLarge,
}

pub fn unreserve_range(address: Page, count: usize) -> Result<(), UnreserveError> {
	util::spin_lock(&GLOBAL.part.reserved_capacity, 0, |capacity| {
		GLOBAL.reserved_entries[..GLOBAL.part.reserved_count.get()]
			.binary_search_by(|e| {
				e.start
					.get()
					.map(|p| p.as_ptr())
					.unwrap_or_else(ptr::null_mut)
					.cmp(&address.as_ptr())
			})
			// TODO check for size
			.map(|i| GLOBAL.reserved_entries[i].start.set(None))
			.map_err(|_| UnreserveError::InvalidAddress)
	})
}

/// Functions & structures intended for `crate::ipc` but defined here because it depends strongly
/// on `GLOBAL`.
pub(crate) mod ipc {

	use super::*;

	/// Error returned when no free slots are available in a queue.
	#[derive(Debug)]
	pub struct NoFreeSlots;

	/// Copy the data from one IPC packet to another packet while ensuring the opcode is written
	/// last.
	fn copy(from: &kernel::ipc::Packet, to: &mut kernel::ipc::Packet) {
		to.uuid = from.uuid;
		to.data = from.data;
		to.offset = from.offset;
		to.length = from.length;
		to.address = from.address;
		to.flags = from.flags;
		to.id = from.id;
		atomic::compiler_fence(Ordering::Release);
		to.opcode = from.opcode;
	}

	/// Send an IPC packet to a task.
	///
	/// This will yield the task if no slots are available.
	pub fn transmit<F, R>(f: F) -> R
	where
		F: FnOnce(&mut kernel::ipc::Packet) -> R,
	{
		// Acquiring the lock before looping should improve performance slightly & reduce
		// the risk of a task stalling indefinitely.
		util::spin_lock(&GLOBAL.part.transmit_mask, 2, |mask| {
			let mut index = GLOBAL.part.transmit_index.get();
			let entries = GLOBAL.part.transmit_entries.get();
			loop {
				match unsafe { get_free_slot(entries, &mut index, *mask) } {
					Ok(pkt) => {
						GLOBAL.part.transmit_index.set(index);
						let mut f_pkt = Default::default();
						let ret = f(&mut f_pkt);
						// Don't increase the index nor copy the data if the
						// opcode is Noe
						if f_pkt.opcode.is_some() {
							GLOBAL.part.transmit_index.set(index);
							copy(&f_pkt, pkt);
						}
						return ret;
					}
					Err(NoFreeSlots) => unsafe {
						kernel::io_wait(0, 0);
					},
				}
			}
		})
	}

	/// Attempt to reserve a slot for sendng an IPC packet to a task.
	pub fn try_transmit<F, R>(f: F) -> Result<R, NoFreeSlots>
	where
		F: FnOnce(&mut kernel::ipc::Packet) -> R,
	{
		util::spin_lock(&GLOBAL.part.transmit_mask, 2, |mask| {
			let mut index = GLOBAL.part.transmit_index.get();
			let entries = GLOBAL.part.transmit_entries.get();
			let pkt = unsafe { get_free_slot(entries, &mut index, *mask) }?;
			let mut f_pkt = Default::default();
			let ret = f(&mut f_pkt);
			// Don't increase the index nor copy the data if the
			// opcode is Noe
			if f_pkt.opcode.is_some() {
				GLOBAL.part.transmit_index.set(index);
				copy(&f_pkt, pkt);
			}
			Ok(ret)
		})
	}

	/// Try to get an unused slot in an IPC queue.
	///
	/// # Safety
	///
	/// The queue must be locked & the lifetime must have an appropriate limit.
	///
	/// The index must be in range.
	unsafe fn get_free_slot<'a>(
		queue: *mut kernel::ipc::Packet,
		index: &mut u16,
		mask: u16,
	) -> Result<&'a mut kernel::ipc::Packet, NoFreeSlots> {
		let entry = &mut *queue.add(usize::from(*index));
		if entry.opcode.is_none() {
			*index += 1;
			*index &= mask;
			Ok(entry)
		} else {
			Err(NoFreeSlots)
		}
	}

	/// Receive an IPC packet.
	///
	/// This will yield the task if no packets have been received yet.
	pub fn receive<F, R>(f: F) -> R
	where
		F: FnOnce(&kernel::ipc::Packet) -> R,
	{
		// Acquiring the lock before looping should improve performance slightly & reduce
		// the risk of a task stalling indefinitely.
		util::spin_lock(&GLOBAL.part.receive_mask, 2, |mask| {
			let mut index = GLOBAL.part.receive_index.get();
			let entries = GLOBAL.part.receive_entries.get();
			loop {
				match unsafe { get_used_slot(entries, &mut index, *mask) } {
					Ok(pkt) => {
						GLOBAL.part.receive_index.set(index);
						let ret = f(pkt);
						pkt.opcode = None;
						return ret;
					}
					Err(NoFreeSlots) => unsafe {
						kernel::io_wait(0, 0);
					},
				}
			}
		})
	}

	/// Attempt to reserve a slot for sendng an IPC packet to a task.
	pub fn try_receive<F, R>(f: F) -> Result<R, NoFreeSlots>
	where
		F: FnOnce(&kernel::ipc::Packet) -> R,
	{
		util::spin_lock(&GLOBAL.part.receive_mask, 2, |mask| {
			let mut index = GLOBAL.part.receive_index.get();
			let entries = GLOBAL.part.receive_entries.get();
			let pkt = unsafe { get_used_slot(entries, &mut index, *mask)? };
			GLOBAL.part.receive_index.set(index);
			let ret = f(pkt);
			pkt.opcode = None;
			Ok(ret)
		})
	}

	/// Try to get an used slot in an IPC queue.
	///
	/// # Safety
	///
	/// The queue must be locked & the lifetime must have an appropriate limit.
	///
	/// The index must be in range.
	unsafe fn get_used_slot<'a>(
		queue: *mut kernel::ipc::Packet,
		index: &mut u16,
		mask: u16,
	) -> Result<&'a mut kernel::ipc::Packet, NoFreeSlots> {
		let entry = &mut *queue.add(usize::from(*index));
		if entry.opcode.is_some() {
			*index += 1;
			*index &= mask;
			Ok(entry)
		} else {
			Err(NoFreeSlots)
		}
	}

	/// Add an address range the kernel is free to map pages into.
	pub fn add_free_range(page: Page, count: usize) -> Result<(), ()> {
		util::spin_lock(&GLOBAL.part.free_ranges_capacity, 0, |capacity| {
			let ranges = unsafe { slice::from_raw_parts_mut(GLOBAL.part.free_ranges.get(), *capacity) };
			// TODO merge fragmented ranges.
			// This should be done by sorting the free range list.
			for (i, range) in ranges.iter_mut().enumerate() {
				if range.count == 0 {
					range.address = Some(page.as_non_null_ptr());
					range.count = count;
					return Ok(());
				}
			}
			Err(())
		})
	}
}
