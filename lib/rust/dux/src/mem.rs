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
use core::ops;
use core::ptr;
use core::ptr::NonNull;
use core::slice;
use core::sync::atomic::{AtomicBool, AtomicU16, AtomicUsize, Ordering};

/// A single memory range.
#[repr(C)]
struct MemoryMap {
	start: Cell<Option<Page>>,
	/// end is *inclusive*, i.e. the offset bits are all `1`s.
	end: Cell<*mut kernel::Page>,
}

/// Part of the global structure but separate to account for be able to account for memory size.
#[repr(C)]
struct GlobalPart {
	/// The table of IPC packets as well as the transmit & receive buffers & free stack.
	ipc_packets: Cell<*mut kernel::ipc::Packet>,

	/// A list of free ranges for use with IPC.
	free_ranges: Cell<*mut kernel::ipc::FreePage>,

	/// A pointer to extra reserved entries.
	extra_reserved_entries: Cell<*mut MemoryMap>,

	/// The amount of occupied reserved entries
	reserved_count: Cell<usize>,

	/// The amount of available reserved entries.
	///
	/// If this is 0, the list is locked.
	reserved_capacity: AtomicUsize,

	/// The amount of available free range entries.
	///
	/// If it is 0, the list is locked.
	free_ranges_capacity: AtomicUsize,

	/// The mask of the ring buffers, which is the length of the buffer `- 1`.
	ring_mask: Cell<u16>,

	/// The slot index of the last processed received packet.
	last_received_index: Cell<u16>,

	/// A lock for the transmit ring buffer.
	///
	/// `false` means the lock is open, `true` means it is locked.
	transmit_lock: AtomicBool,

	/// A lock for the received ring buffer.
	///
	/// `false` means the lock is open, `true` means it is locked.
	received_lock: AtomicBool,
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
#[export_name = "__dux_init"]
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

	// Set up IPC queues
	let packets_count = 4;

	// Reserve pages for IPC
	// FIXME handle errors properly
	let addr = reserve_range(None, 8).unwrap();
	let ret = kernel::mem_alloc(addr.as_ptr(), 1, kernel::PROT_READ_WRITE);
	if ret.status != 0 {
		// FIXME handle errors properly
		todo!()
	}

	GLOBAL
		.part
		.ipc_packets
		.set(addr.as_ptr().cast::<kernel::ipc::Packet>());
	GLOBAL.part.last_received_index.set(0);
	GLOBAL.part.ring_mask.set(packets_count - 1);

	// Push the slots on the free stack
	for slot in 0..packets_count {
		ipc::push_free_slot(slot);
	}

	// Reserve pages for free ranges
	// FIXME handle errors properly
	let addr = reserve_range(None, 8).unwrap();
	let ret = kernel::mem_alloc(addr.as_ptr(), 1, kernel::PROT_READ_WRITE);
	if ret.status != 0 {
		// FIXME handle errors properly
		todo!()
	}
	let free_ranges = addr.as_ptr().cast();
	let free_ranges_len = 12;
	GLOBAL.part.free_ranges.set(free_ranges);
	GLOBAL
		.part
		.free_ranges_capacity
		.store(free_ranges_len, Ordering::Release);

	// Set a range to which pages can be mapped to.
	let free_ranges = slice::from_raw_parts_mut(free_ranges, free_ranges_len);
	let count = free_ranges_len; // The effective maximum right now.
	let addr = reserve_range(None, count).unwrap();
	ipc::add_free_range(addr, count).unwrap();

	// Register the queues to the kernel
	let ret = kernel::io_set_queues(
		GLOBAL.part.ipc_packets.get(),
		(packets_count - 1).count_ones() as u8,
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
		if let Some(_address) = address {
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

pub fn unreserve_range(address: Page, _count: usize) -> Result<(), UnreserveError> {
	util::spin_lock(&GLOBAL.part.reserved_capacity, 0, |_capacity| {
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

	/// Send an IPC packet to a task.
	///
	/// This will yield the task if no slots are available.
	pub fn transmit() -> TransmitLock {
		let _ = util::SpinLockGuard::new(&GLOBAL.part.transmit_lock, true).into_raw();
		loop {
			match pop_free_slot() {
				Ok(slot) => return TransmitLock { slot },
				Err(NoFreeSlots) => unsafe {
					let _ = kernel::io_wait(u64::MAX);
				},
			}
		}
	}

	/// Attempt to reserve a slot for sendng an IPC packet to a task.
	pub fn try_transmit() -> Result<TransmitLock, NoFreeSlots> {
		let guard = util::SpinLockGuard::new(&GLOBAL.part.transmit_lock, true);
		let slot = pop_free_slot()?;
		let _ = guard.into_raw();
		Ok(TransmitLock { slot })
	}

	/// A lock on the transmit queue along with the slot of the packet to write to.
	pub struct TransmitLock {
		slot: u16,
	}

	impl TransmitLock {
		pub fn into_raw<'a>(self) -> (u16, &'a mut kernel::ipc::Packet) {
			let slot = self.slot;
			mem::forget(self);
			(slot, unsafe { packet(slot) }.unwrap())
		}

		pub unsafe fn from_raw(slot: u16) -> Self {
			Self { slot }
		}
	}

	impl ops::Deref for TransmitLock {
		type Target = kernel::ipc::Packet;

		fn deref(&self) -> &Self::Target {
			unsafe { packet(self.slot) }.unwrap()
		}
	}

	impl ops::DerefMut for TransmitLock {
		fn deref_mut(&mut self) -> &mut Self::Target {
			unsafe { packet(self.slot) }.unwrap()
		}
	}

	impl Drop for TransmitLock {
		fn drop(&mut self) {
			let (index, entries) = unsafe { transmit_ring() };
			let mask = GLOBAL.part.ring_mask.get();

			entries[usize::from(*index & mask)] = self.slot;
			*index = index.wrapping_add(1);

			unsafe { util::SpinLockGuard::from_raw(&GLOBAL.part.transmit_lock, false) };
		}
	}

	/// Receive an IPC packet.
	///
	/// This will yield the task if no packets have been received yet.
	pub fn receive() -> ReceivedLock {
		let _ = util::SpinLockGuard::new(&GLOBAL.part.received_lock, true).into_raw();
		let mask = GLOBAL.part.ring_mask.get();
		loop {
			let (index, entries) = unsafe { received_ring() };
			let i = GLOBAL.part.last_received_index.get();
			if i != index {
				return ReceivedLock {
					slot: entries[usize::from(i & mask)].get(),
				};
			}
			let _ = unsafe { kernel::io_wait(u64::MAX) };
		}
	}

	/// Attempt to reserve a slot for sendng an IPC packet to a task.
	pub fn try_receive() -> Option<ReceivedLock> {
		let guard = util::SpinLockGuard::new(&GLOBAL.part.received_lock, true);

		let (index, entries) = unsafe { received_ring() };
		let mask = GLOBAL.part.ring_mask.get();
		let i = GLOBAL.part.last_received_index.get();
		(i != index).then(|| {
			let _ = guard.into_raw();
			ReceivedLock {
				slot: entries[usize::from(i & mask)].get(),
			}
		})
	}

	/// A lock on the received queue along with the slot of the packet to write to.
	pub struct ReceivedLock {
		slot: u16,
	}

	impl ReceivedLock {
		pub fn into_raw<'a>(self) -> (u16, &'a kernel::ipc::Packet) {
			let slot = self.slot;
			mem::forget(self);
			(slot, unsafe { packet(slot) }.unwrap())
		}

		pub unsafe fn from_raw(slot: u16) -> Self {
			Self { slot }
		}

		/// Release the lock but don't discard the packet. Instead, swap the packet with the last
		/// available entry in the ring buffer.
		pub fn defer(self) {
			let (index, entries) = unsafe { received_ring() };
			let last_index = GLOBAL.part.last_received_index.get();
			let mask = GLOBAL.part.ring_mask.get();

			let a = entries[usize::from(last_index & mask)].get();
			let b = entries[usize::from(index & mask)].get();
			debug_assert_eq!(a, self.slot, "current received entry mutated while locked");
			assert_eq!(a, self.slot, "current received entry mutated while locked");
			entries[usize::from(index & mask)].set(a);
			entries[usize::from(last_index & mask)].set(b);

			unsafe { util::SpinLockGuard::from_raw(&GLOBAL.part.received_lock, false) };
		}
	}

	impl ops::Deref for ReceivedLock {
		type Target = kernel::ipc::Packet;

		fn deref(&self) -> &Self::Target {
			unsafe { packet(self.slot) }.unwrap()
		}
	}

	impl Drop for ReceivedLock {
		fn drop(&mut self) {
			let i = GLOBAL.part.last_received_index.get();
			GLOBAL.part.last_received_index.set(i.wrapping_add(1));
			drop(unsafe { util::SpinLockGuard::from_raw(&GLOBAL.part.received_lock, false) });
			// push it after dropping the lock to reduce contention
			unsafe { push_free_slot(self.slot) };
		}
	}

	/// Add an address range the kernel is free to map pages into.
	pub fn add_free_range(page: Page, count: usize) -> Result<(), ()> {
		util::spin_lock(&GLOBAL.part.free_ranges_capacity, 0, |capacity| {
			let ranges =
				unsafe { slice::from_raw_parts_mut(GLOBAL.part.free_ranges.get(), *capacity) };
			// FIXME return an error if a double page is being added.
			ranges
				.iter()
				.filter_map(|r| (r.count > 0).then(|| r.address))
				.for_each(|addr| assert_ne!(addr, Some(page.as_non_null_ptr())));
			// TODO merge fragmented ranges.
			// This should be done by sorting the free range list.
			for range in ranges.iter_mut() {
				if range.count == 0 {
					range.address = Some(page.as_non_null_ptr());
					range.count = count;
					return Ok(());
				}
			}
			Err(())
		})
	}

	/// Return the IPC packet at a given slot.
	///
	/// # Safety
	///
	/// The queue may not be resized while there is a reference to the packet.
	///
	/// There may be no other references to this packet.
	unsafe fn packet<'a>(index: u16) -> Option<&'a mut kernel::ipc::Packet> {
		(index <= GLOBAL.part.ring_mask.get())
			.then(|| &mut *GLOBAL.part.ipc_packets.get().add(usize::from(index)))
	}

	/// Return the transmit index & buffer.
	///
	/// # Safety
	///
	/// There may be no other references to this buffer.
	///
	/// The ring may not be resized while there is a reference to the slice.
	///
	/// The ring must be locked during this call.
	unsafe fn transmit_ring<'a>() -> (&'a mut u16, &'a mut [u16]) {
		let len = usize::from(ring_len());
		// Skip table
		let addr = GLOBAL.part.ipc_packets.get().add(len).cast::<u16>();
		let index = &mut *addr;
		let slice = slice::from_raw_parts_mut(addr.cast::<u16>().add(1), len);
		(index, slice)
	}

	/// Return the received index & buffer.
	///
	/// # Safety
	///
	/// The ring may not be resized while there is a reference to the slice.
	///
	/// The ring must be locked during this call.
	unsafe fn received_ring<'a>() -> (u16, &'a [Cell<u16>]) {
		let len = usize::from(ring_len());
		// Skip table + transmit ring
		// Use an AtomicU16 as the kernel may write to it from another thread.
		let addr = GLOBAL
			.part
			.ipc_packets
			.get()
			.add(len)
			.cast::<AtomicU16>()
			.add(1 + len);
		let index = (&*addr).load(Ordering::Acquire);
		let slice = slice::from_raw_parts(addr.cast::<Cell<u16>>().add(1), len);
		(index, slice)
	}

	/// Try to get an unused slot from the free stack.
	fn pop_free_slot() -> Result<u16, NoFreeSlots> {
		let (top, entries) = unsafe { free_stack() };
		util::spin_lock(top, u16::MAX, |top| {
			top.checked_sub(1)
				.map(|t| {
					*top = t;
					entries[usize::from(t)].get()
				})
				.ok_or(NoFreeSlots)
		})
	}

	/// Add an unused slot to the free stack.
	///
	/// # Safety
	///
	/// The index must be in range and not already present on the stack.
	pub(super) unsafe fn push_free_slot(slot: u16) {
		let (top, entries) = free_stack();
		util::spin_lock(top, u16::MAX, |top| {
			assert!(*top < ring_len(), "free stack overflow");
			entries[usize::from(*top)].set(slot);
			*top += 1;
		});
	}

	/// Return the free stack.
	///
	/// # Safety
	///
	/// The stack may not be resized while there is a reference to the slice.
	///
	/// The stack must be locked during this call.
	unsafe fn free_stack<'a>() -> (&'a AtomicU16, &'a [Cell<u16>]) {
		let len = usize::from(ring_len());
		// Skip table + transmit ring
		// Use an AtomicU16 as the kernel may write to it from another thread.
		let addr = GLOBAL
			.part
			.ipc_packets
			.get()
			.add(len)
			.cast::<AtomicU16>()
			.add((1 + len) * 2);
		let top = &*addr;
		let slice = slice::from_raw_parts_mut(addr.cast::<Cell<u16>>().add(1), len);
		(top, slice)
	}

	/// Returns the length of the ring buffers.
	///
	/// The queue may not be resized during this call.
	unsafe fn ring_len() -> u16 {
		debug_assert_ne!(GLOBAL.part.ring_mask.get(), u16::MAX);
		GLOBAL.part.ring_mask.get() + 1
	}
}
