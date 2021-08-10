//! Structures & functions to facilitate inter-task communication

use super::group::Group;
use super::Address;
use crate::arch::{self, Page, PageData};
use core::cell::Cell;
use core::num::NonZeroU8;
use core::ptr::NonNull;
use core::slice;
use core::sync::atomic::{AtomicU16, Ordering};

/// An IPC packet.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Packet {
	uuid: [u64; 2],
	data: Option<NonNull<PageData>>,
	name: Option<NonNull<PageData>>,
	data_offset: u64,
	data_length: usize,
	address: Address,
	flags: Flags,
	name_length: u16,
	id: u8,
	opcode: Option<NonZeroU8>,
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

#[derive(Debug)]
struct InvalidOp;

#[derive(Debug)]
pub struct TooLarge;

#[derive(Debug)]
enum PopFreeSlotError {
	LockTimeout,
	NoFreeSlots,
}

#[derive(Debug)]
enum PushFreeSlotError {
	LockTimeout,
	Full,
}

/// A structure used for handling IPC.
pub struct IPC {
	/// The address of the packets buffer.
	packets: NonNull<Packet>,
	/// The index of the last processed transmit entry.
	last_transmit_index: Cell<u16>,
	/// The mask of the ring buffers, which is `packet_count - 1`.
	ring_mask: u16,
	/// A list of address that can be freely mapped for IPC.
	free_pages: NonNull<FreePage>,
	/// The maximum amount of free pages.
	max_free_pages: usize,
}

impl IPC {
	/// Create a new IPC structure.
	///
	/// # Safety
	///
	/// The address *must* point to user-owned memory and may never point to kernel-owned
	/// memory.
	///
	/// # Returns
	///
	/// `Err(TooLarge)` if `mask_bits` is larger than `15`.
	pub unsafe fn new(
		packets: NonNull<Packet>,
		mask_bits: u8,
		free_pages: NonNull<FreePage>,
		max_free_pages: usize,
	) -> Result<Self, TooLarge> {
		(mask_bits <= 15)
			.then(|| Self {
				packets,
				last_transmit_index: Cell::new(0),
				ring_mask: (1 << mask_bits) - 1,
				free_pages,
				max_free_pages,
			})
			.ok_or(TooLarge)
	}

	/// Process IPC packets to be transmitted.
	pub fn process_packets(&mut self, slf_task: &super::Task, slf_address: Address) {
		use crate::arch::vms::VirtualMemorySystem;
		slf_task.inner().shared_state.virtual_memory.activate(); // TODO do this beforehand and once only
		arch::set_supervisor_userpage_access(true);
		let (tx_index, tx_slots) = self.transmit_ring();
		let mut last_transmit_index = self.last_transmit_index.get();
		while last_transmit_index != tx_index {
			let tx_pkt_slot = last_transmit_index & self.ring_mask;
			let tx_pkt = unsafe { *self.packet(tx_slots[usize::from(tx_pkt_slot)]).unwrap() };

			// Disallow sending packets to self since it's pointless + leads to potential aliasing
			// bugs.
			assert_ne!(tx_pkt.address, slf_address, "can't transmit to self");

			let (group, task) = (tx_pkt.address.group(), tx_pkt.address.task());
			let task = Group::get(group.into()).unwrap().task(task.into()).unwrap();

			// TODO this is potentially terribly inefficient
			//
			// If ASIDs are available, then the impact is likely _okay_, but if there are
			// no ASIDs then this has a huge context switching cost.
			//
			// It may be worth mapping the packet tables into kernel space.
			task.inner().shared_state.virtual_memory.activate();

			let task_ipc = task.inner().ipc.as_mut().unwrap();
			let (rx_index, rx_slots) = task_ipc.received_ring();
			let rx_pkt_slot = task_ipc.pop_free_slot().unwrap();

			// Get address range to map the data
			let tx_rx_data = tx_pkt.data.map(|data| {
				let page = Page::new(data).unwrap();
				let count = Page::min_pages_for_byte_count(tx_pkt.data_length);
				(page, task_ipc.pop_free_range(count).unwrap(), count)
			});

			// Get address range to map the name
			let tx_rx_name = tx_pkt.name.map(|name| {
				let page = Page::new(name).unwrap();
				let count = Page::min_pages_for_byte_count(usize::from(tx_pkt.name_length));
				(page, task_ipc.pop_free_range(count).unwrap(), count)
			});

			rx_slots[usize::from(rx_index.load(Ordering::Acquire) & task_ipc.ring_mask)]
				.set(rx_pkt_slot);

			let rx_pkt = unsafe { task_ipc.packet(rx_pkt_slot).unwrap() };

			*rx_pkt = Packet {
				uuid: tx_pkt.uuid,
				data: tx_rx_data.map(|(_, p, _)| p.as_non_null_ptr()),
				name: tx_rx_name.map(|(_, p, _)| p.as_non_null_ptr()),
				data_length: tx_pkt.data_length,
				data_offset: tx_pkt.data_offset,
				name_length: tx_pkt.name_length,
				address: slf_address,
				flags: tx_pkt.flags,
				opcode: tx_pkt.opcode,
				id: tx_pkt.id,
			};

			rx_index.fetch_add(1, Ordering::Release);

			// TODO ditto
			slf_task.inner().shared_state.virtual_memory.activate();
			let vm = &mut task.inner().shared_state.virtual_memory;

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

			self.push_free_slot(tx_pkt_slot).unwrap();

			last_transmit_index = last_transmit_index.wrapping_add(1);
		}
		self.last_transmit_index.set(last_transmit_index);
		arch::set_supervisor_userpage_access(false);
	}

	/// Pop an address range from the free ranges list.
	fn pop_free_range(&self, size: usize) -> Option<Page> {
		let free_pages =
			unsafe { slice::from_raw_parts_mut(self.free_pages.as_ptr(), self.max_free_pages) };
		for fp in free_pages.iter_mut() {
			if let Some(addr) = fp.address {
				if let Some(remaining) = fp.count.checked_sub(size) {
					fp.count = remaining;
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
	unsafe fn packet(&self, index: u16) -> Option<&mut Packet> {
		(index <= self.ring_mask).then(|| &mut *self.packets.as_ptr().add(usize::from(index)))
	}

	/// Return the transmit ring buffer list.
	#[must_use]
	fn transmit_ring(&self) -> (u16, &[u16]) {
		unsafe {
			let count = usize::from(self.len());
			let addr = self.packets.as_ptr().add(count).cast::<u16>();
			let index = *addr;
			let slice = slice::from_raw_parts(addr.add(1), count);
			(index, slice)
		}
	}

	/// Return the received ring buffer list.
	// FIXME lock the ring.
	#[must_use]
	fn received_ring(&self) -> (&AtomicU16, &[Cell<u16>]) {
		unsafe {
			let count = usize::from(self.len());
			let addr = self
				.packets
				.as_ptr()
				.add(count)
				.cast::<AtomicU16>()
				.add(1 + count);
			let index = &*addr;
			let slice = slice::from_raw_parts(addr.add(1).cast::<Cell<u16>>(), count);
			(index, slice)
		}
	}

	/// Get a free slot.
	fn pop_free_slot(&self) -> Result<u16, PopFreeSlotError> {
		self.lock_free_stack(|top, stack| {
			top.checked_sub(1)
				.map(|tv| {
					*top = tv;
					stack[usize::from(tv)].get()
				})
				.ok_or(PopFreeSlotError::NoFreeSlots)
		})
		.unwrap_or(Err(PopFreeSlotError::LockTimeout))
	}

	/// Add a free slot.
	fn push_free_slot(&self, slot: u16) -> Result<(), PushFreeSlotError> {
		self.lock_free_stack(|top, stack| {
			(*top <= self.ring_mask)
				.then(|| {
					stack[usize::from(*top)].set(slot);
					*top += 1;
				})
				.ok_or(PushFreeSlotError::Full)
		})
		.unwrap_or(Err(PushFreeSlotError::LockTimeout))
	}

	/// Try to lock the stack.
	#[must_use]
	fn lock_free_stack<F, R>(&self, f: F) -> Option<R>
	where
		F: FnOnce(&mut u16, &[Cell<u16>]) -> R,
	{
		let count = usize::from(self.len());
		let addr = unsafe {
			self.packets
				.as_ptr()
				.add(count)
				.cast::<AtomicU16>()
				.add((1 + count) * 2)
		};
		let lock = unsafe { &*addr };
		let stack = unsafe { slice::from_raw_parts(addr.add(1).cast::<Cell<u16>>(), count) };

		let mut val = lock.load(Ordering::Acquire);
		let mut counter = 30usize;
		loop {
			// Wait until the lock is released.
			while val == u16::MAX {
				counter = counter.checked_sub(1)?;
				val = lock.load(Ordering::Acquire);
			}

			// Try to acquire the lock
			match lock.compare_exchange_weak(val, u16::MAX, Ordering::Acquire, Ordering::Acquire) {
				Ok(mut v) => {
					let ret = f(&mut v, stack);
					lock.store(v, Ordering::Release);
					return Some(ret);
				}
				Err(v) => val = v,
			}
		}
	}

	/// The amount of packets in the table.
	pub fn len(&self) -> u16 {
		debug_assert!(self.ring_mask.checked_add(1).is_some(), "length overflowed");
		self.ring_mask + 1
	}
}

/// A single free page.
#[derive(Debug)]
#[repr(C)]
pub struct FreePage {
	address: Option<NonNull<PageData>>,
	count: usize,
}

impl super::Task {
	/// Process IPC packets to be transmitted.
	pub fn process_io(&self, slf_address: Address) {
		self.inner()
			.ipc
			.as_mut()
			.map(|ipc| ipc.process_packets(self, slf_address));
	}
}
