//! Structures & functions to facilitate inter-task communication

use super::*;
use crate::arch::vms::VirtualMemorySystem;
use crate::arch::{self, Page, PageData};
use core::cell::Cell;
use core::convert::TryInto;
use core::mem;
use core::num::NonZeroU8;
use core::ptr::{self, NonNull};
use core::slice;
use core::sync::atomic::{AtomicPtr, AtomicU32, AtomicUsize, Ordering};

/// An IPC packet.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Packet {
	data: Option<NonNull<PageData>>,
	name: Option<NonNull<PageData>>,
	data_offset: u64,
	data_length: u32,
	address: TaskID,
	flags_user: u16,
	flags_kernel: Flags,
	name_length: u16,
	id: u16,
}

/// IPC packet flags
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct Flags(u16);

impl Flags {
	const READABLE: u16 = 0x1;
	const WRITEABLE: u16 = 0x2;
	const EXECUTABLE: u16 = 0x4;
	const LOCK: u16 = 0x8;

	#[must_use]
	#[allow(dead_code)]
	pub fn readable(&self) -> bool {
		self.0 & Self::READABLE > 0
	}

	#[must_use]
	#[allow(dead_code)]
	pub fn writeable(&self) -> bool {
		self.0 & Self::WRITEABLE > 0
	}

	#[must_use]
	#[allow(dead_code)]
	pub fn executable(&self) -> bool {
		self.0 & Self::EXECUTABLE > 0
	}

	#[must_use]
	#[allow(dead_code)]
	pub fn lock(&self) -> bool {
		self.0 & Self::LOCK > 0
	}
}

#[repr(C)]
struct Indices {
	transmit_queue_index: AtomicU32,
	received_queue_index: AtomicU32,
	free_packets_queue_index: AtomicU32,
	free_ranges_list_size: AtomicU32,
}

/// A structure used for handling IPC.
///
/// Everything is laid out contiguously in memory, so only one pointer is necessary.
#[derive(Default)]
#[repr(C)]
pub struct IPC {
	/// The base address of the shared IPC structure.
	///
	/// If no packets queue is specified, this is null.
	base: Cell<Option<NonNull<u8>>>,
	/// The index of the last processed transmit entry.
	last_transmit_index: Cell<u16>,
	/// The index of the last free packet slot entry.
	last_free_index: Cell<u16>,
	/// The mask of the ring buffers, which is `packet_count - 1`.
	ring_mask: Cell<u16>,
}

impl super::Task {
	/// Process IPC packets to be transmitted.
	pub fn process_io(&self, slf_address: TaskID) {
		self.lock_transmit_queue();

		let base = match self.ipc.base.get() {
			Some(base) => base,
			None => return, // Nothing to do
		};

		let mask = self.ipc.ring_mask.get();

		self.virtual_memory.activate(); // TODO do this beforehand and once only
		arch::set_supervisor_userpage_access(true);

		let (tx_index, tx_slots) = IPC::transmit_queue(base, mask);

		let mut last_transmit_index = self.ipc.last_transmit_index.get();

		while last_transmit_index != tx_index {
			// Get a reference to the packet.
			let tx_pkt_slot = last_transmit_index & mask;
			let tx_pkt_slot = tx_slots[usize::from(tx_pkt_slot)].get();
			let tx_pkt = match unsafe { IPC::packet(base, mask, tx_pkt_slot) } {
				Some(pkt) => *pkt,
				// "Get stuck" so the application developer hopefully has a quicker & easier time
				// figuring out why no packets are transmitting.
				None => break,
			};

			let mut fail = || {
				// Mark the packet as undelivered and put in received queue.
				// TODO reserve flag bit
				self.lock_received_queue();
				let (rx_index, rx_slots) = unsafe { IPC::received_queue(base, mask) };
				let i = rx_index.load(Ordering::Relaxed);
				rx_slots[usize::from(i as u16 & mask)].set(tx_pkt_slot);
				rx_index.store(i.wrapping_add(1), Ordering::Release);
				last_transmit_index = last_transmit_index.wrapping_add(1);
			};

			// Get the task to send the packet to.
			let task = match super::get(tx_pkt.address) {
				Some(task) => task,
				None => {
					fail();
					continue;
				}
			};

			task.lock_received_queue();
			let rx_base = match task.ipc.base.get() {
				Some(rx_base) => rx_base,
				None => {
					fail();
					continue;
				}
			};
			let rx_mask = task.ipc.ring_mask.get();

			task.virtual_memory.activate();

			// Get a free packet slot
			let (free_index, free_slots) = unsafe { IPC::free_queue(rx_base, rx_mask) };
			let index = task.ipc.last_free_index.get();
			if free_index.load(Ordering::Acquire) as u16 == index {
				fail();
				continue;
			}
			let rx_pkt_slot = free_slots[usize::from(index)].get();
			let rx_pkt = match unsafe { IPC::packet(rx_base, rx_mask, rx_pkt_slot) } {
				Some(pkt) => pkt,
				None => {
					fail();
					continue;
				}
			};
			task.ipc.last_free_index.set(index.wrapping_add(1));

			// Helper to map data & name
			let map_range = |addr: Option<_>, len| {
				addr.map(|addr| {
					let page = Page::new(addr).map_err(|_| ())?;
					let count = Page::min_pages_for_byte_count(len);
					let range = IPC::take_free_range(rx_base, rx_mask, count).ok_or(())?;
					Ok((page, range, count))
				})
				.transpose()
			};

			// Get address range to map the data
			let tx_rx_data = match map_range(tx_pkt.data, tx_pkt.data_length.try_into().unwrap()) {
				Ok(data) => data,
				Err(()) => {
					fail();
					continue;
				}
			};

			// Get address range to map the name
			let tx_rx_name = match map_range(tx_pkt.name, tx_pkt.name_length.into()) {
				Ok(name) => name,
				Err(()) => {
					fail();
					continue;
				}
			};

			// Fill out the packet data
			*rx_pkt = Packet {
				data: tx_rx_data.map(|(_, p, _)| p.as_non_null_ptr()),
				name: tx_rx_name.map(|(_, p, _)| p.as_non_null_ptr()),
				data_length: tx_pkt.data_length,
				data_offset: tx_pkt.data_offset,
				name_length: tx_pkt.name_length,
				address: slf_address,
				flags_user: tx_pkt.flags_user,
				flags_kernel: tx_pkt.flags_kernel,
				id: tx_pkt.id,
			};

			// Make packet available
			let (rx_index, rx_slots) = unsafe { IPC::received_queue(rx_base, rx_mask) };
			let index = rx_index.load(Ordering::Acquire);
			rx_slots[usize::from(index as u16 & rx_mask)].set(rx_pkt_slot);
			rx_index.store(index.wrapping_add(1), Ordering::Release);

			// Clear the tasks wait time so it will be rescheduled
			task.wait_time.store(0, Ordering::Relaxed);

			// TODO ditto
			self.virtual_memory.activate();
			let vm = &task.virtual_memory;

			// FIXME map the entire range of pages instead of just one.
			if let Some((tx_data, rx_data, count)) = tx_rx_data {
				for i in 0..count {
					vm.share(
						rx_data.skip(i).unwrap(),
						tx_data.skip(i).unwrap(),
						arch::vms::RWX::RW, // TODO use flags field for this
						arch::vms::Accessibility::UserLocal,
					)
					.unwrap();
				}
			}
			if let Some((tx_name, rx_name, count)) = tx_rx_name {
				for i in 0..count {
					vm.share(
						rx_name.skip(i).unwrap(),
						tx_name.skip(i).unwrap(),
						arch::vms::RWX::R,
						arch::vms::Accessibility::UserLocal,
					)
					.unwrap();
				}
			}

			last_transmit_index = last_transmit_index.wrapping_add(1);
		}
		self.ipc.last_transmit_index.set(last_transmit_index);
		arch::set_supervisor_userpage_access(false);
	}
}

impl IPC {
	/// Take an address range from the free ranges list.
	fn take_free_range(base: NonNull<u8>, mask: u16, size: usize) -> Option<Page> {
		let base = base.as_ptr();
		let free_pages_count = unsafe {
			(&*base.add(Self::queue_indices_offset(mask)).cast::<Indices>())
				.free_ranges_list_size
				.load(Ordering::Relaxed);
		};
		let free_pages = unsafe {
			let free_pages = base.add(Self::free_ranges_offset(mask)).cast::<FreePage>();
			slice::from_raw_parts_mut(free_pages, usize::from(mask) + 1)
		};
		for fp in free_pages.iter_mut() {
			// FIXME there are potential race conditions here.
			//
			// One potential fix is to lock the address by setting it to null, do whatever else
			// and restore the original address after.
			if let Some(addr) = NonNull::new(fp.address.load(Ordering::Relaxed)) {
				if let Some(remaining) = fp.count.load(Ordering::Relaxed).checked_sub(size) {
					fp.count.store(remaining, Ordering::Relaxed);
					return Page::new(addr).ok().and_then(|p| p.skip(remaining));
				}
			}
		}
		None
	}

	/// Return a mutable reference to a packet in a certain slot.
	///
	/// # Safety
	///
	/// There may only be one mutable reference to a packet at any time.
	#[must_use]
	unsafe fn packet<'a>(base: NonNull<u8>, mask: u16, index: u16) -> Option<&'a mut Packet> {
		(index <= mask).then(|| {
			let pkt = base
				.as_ptr()
				.add(Self::packets_table_offset(mask))
				.cast::<Packet>();
			&mut *pkt.add(usize::from(index))
		})
	}

	/// Return the transmit queue.
	#[must_use]
	fn transmit_queue<'a>(base: NonNull<u8>, mask: u16) -> (u16, &'a [Cell<u16>]) {
		let base = base.as_ptr();
		let index = unsafe { &*base.add(Self::queue_indices_offset(mask)).cast::<Indices>() };
		let index = index.transmit_queue_index.load(Ordering::Acquire) as u16;
		let slice = base
			.wrapping_add(Self::transmit_queue_offset(mask))
			.cast::<Cell<u16>>();
		let slice = unsafe { slice::from_raw_parts(slice, usize::from(mask) + 1) };
		(index, slice)
	}

	/// Return the received queue.
	///
	/// # Safety
	///
	/// The queue must be locked somehow.
	#[must_use]
	unsafe fn received_queue<'a>(base: NonNull<u8>, mask: u16) -> (&'a AtomicU32, &'a [Cell<u16>]) {
		let base = base.as_ptr();
		let index = unsafe { &*base.add(Self::queue_indices_offset(mask)).cast::<Indices>() };
		let index = &index.received_queue_index;
		let slice = base
			.wrapping_add(Self::received_queue_offset(mask))
			.cast::<Cell<u16>>();
		let slice = unsafe { slice::from_raw_parts(slice, usize::from(mask) + 1) };
		(index, slice)
	}

	/// Return the free slot queue.
	///
	/// # Safety
	///
	/// The queue must be locked somehow.
	#[must_use]
	unsafe fn free_queue<'a>(base: NonNull<u8>, mask: u16) -> (&'a AtomicU32, &'a [Cell<u16>]) {
		let base = base.as_ptr();
		let index = unsafe { &*base.add(Self::queue_indices_offset(mask)).cast::<Indices>() };
		let index = &index.free_packets_queue_index;
		let slice = base
			.wrapping_add(Self::free_packets_queue_offset(mask))
			.cast::<Cell<u16>>();
		let slice = unsafe { slice::from_raw_parts(slice, usize::from(mask) + 1) };
		(index, slice)
	}

	/// Determine the size of the packets table in bytes with the given mask.
	fn packets_table_byte_size(mask: u16) -> usize {
		(usize::from(mask) + 1) * mem::size_of::<Packet>()
	}

	/// Determine the size of a single queue (ring buffer).
	///
	/// The transmit, received and free buffers use the same structure and the size of each can
	/// be determined with this function.
	fn queue_byte_size(mask: u16) -> usize {
		mem::size_of::<AtomicU16>() + (usize::from(mask) + 1) * mem::size_of::<u16>()
	}

	/// Determine the size of the free stack.
	fn free_ranges_byte_size(mask: u16) -> usize {
		mem::size_of::<AtomicU16>() + (usize::from(mask) + 1) * mem::size_of::<FreePage>()
	}

	/// The offset of the packets table in bytes.
	fn packets_table_offset(_mask: u16) -> usize {
		0
	}

	/// The offset of the free ranges list.
	fn free_ranges_offset(mask: u16) -> usize {
		Self::packets_table_offset(mask) + Self::packets_table_byte_size(mask)
	}

	/// The offset of the `Indices` struct.
	fn queue_indices_offset(mask: u16) -> usize {
		Self::free_ranges_offset(mask) + Self::free_ranges_byte_size(mask)
	}

	/// The offset of the transmit queue.
	fn transmit_queue_offset(mask: u16) -> usize {
		Self::queue_indices_offset(mask) + mem::size_of::<Indices>()
	}

	/// The offset of the receive queue.
	fn received_queue_offset(mask: u16) -> usize {
		Self::transmit_queue_offset(mask) + Self::queue_byte_size(mask)
	}

	/// The offset of the free queue.
	fn free_packets_queue_offset(mask: u16) -> usize {
		Self::received_queue_offset(mask) + Self::queue_byte_size(mask)
	}
}

/// A free address range.
#[repr(C)]
pub struct FreePage {
	address: AtomicPtr<PageData>,
	count: AtomicUsize,
}

struct TransmitLock<'a>(&'a Task);

impl Drop for TransmitLock<'_> {
	fn drop(&mut self) {
		unsafe {
			self.0.unlock_transmit_queue();
		}
	}
}

struct ReceivedLock<'a>(&'a Task);

impl Drop for ReceivedLock<'_> {
	fn drop(&mut self) {
		unsafe {
			self.0.unlock_received_queue();
		}
	}
}

impl super::Task {
	fn lock_transmit_queue(&self) -> TransmitLock {
		self.flags.lock(super::Flags::IPC_LOCK_TRANSMIT);
		TransmitLock(self)
	}

	unsafe fn unlock_transmit_queue(&self) {
		self.flags.unlock(super::Flags::IPC_LOCK_TRANSMIT);
	}

	fn lock_received_queue(&self) -> ReceivedLock {
		self.flags.lock(super::Flags::IPC_LOCK_RECEIVED);
		ReceivedLock(self)
	}

	unsafe fn unlock_received_queue(&self) {
		self.flags.unlock(super::Flags::IPC_LOCK_RECEIVED);
	}

	/// Set the task transmit & receive queue pointers and sizes.
	pub fn set_queues(&self, base: Option<NonNull<u8>>, bits: u8) -> Result<(), SetQueuesError> {
		// Limit to 1024 slots for now.
		(bits <= 10).then(|| ()).ok_or(SetQueuesError::TooLarge)?;
		let flags = super::Flags::IPC_LOCK_TRANSMIT | super::Flags::IPC_LOCK_RECEIVED;
		self.flags.lock(flags);
		self.ipc.base.set(base);
		self.ipc.ring_mask.set((1 << bits) - 1);
		self.ipc.last_transmit_index.set(0);
		self.ipc.last_free_index.set(0);
		self.flags.unlock(flags);
		Ok(())
	}
}

pub enum SetQueuesError {
	TooLarge,
}
