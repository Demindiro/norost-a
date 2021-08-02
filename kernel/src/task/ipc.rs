//! Structures & functions to facilitate inter-task communication

use super::group::Group;
use crate::arch::{self, Page};
use core::convert::TryFrom;
use core::mem;
use core::num::NonZeroU8;
use core::ptr::NonNull;

/// Structure representing an index and mask in a ring buffer.
pub struct RingIndex {
	mask: u16,
	index: u16,
}

impl RingIndex {
	pub fn new(bits: u8) -> Self {
		Self {
			mask: (1 << bits) - 1,
			index: 0,
		}
	}

	#[inline(always)]
	pub fn set_mask(&mut self, mask: u8) {
		let mask = (1 << mask) - 1;
		self.index &= mask;
		self.mask = mask;
	}

	pub fn increment(&mut self) {
		self.index += 1;
		self.index &= self.mask;
	}

	pub fn get(&self) -> usize {
		self.index.into()
	}
}

impl Default for RingIndex {
	fn default() -> Self {
		Self { mask: 0, index: 0 }
	}
}

/// Union of all possible data types that can be pointed to in a packet's `data` field.
///
/// All members are pointers.
#[derive(Clone, Copy)]
union Data {
	/// An address range with raw data to be read or to which data should be written.
	raw: *mut u8,
}

/// An IPC packet.
#[repr(C)]
#[derive(Clone)]
pub struct Packet {
	opcode: Option<NonZeroU8>,
	priority: i8,
	flags: u16,
	id: u32,
	address: usize,
	length: usize,
	data: Data,
}

/// A decoded opcode
#[repr(u8)]
enum Op {
	Read = 1,
	Write = 2,
}

impl TryFrom<NonZeroU8> for Op {
	type Error = InvalidOp;

	fn try_from(n: NonZeroU8) -> Result<Self, Self::Error> {
		Ok(match n.get() {
			1 => Op::Read,
			2 => Op::Write,
			_ => Err(InvalidOp)?,
		})
	}
}

impl From<Op> for NonZeroU8 {
	fn from(op: Op) -> Self {
		NonZeroU8::new(match op {
			Op::Read => 1,
			Op::Write => 2,
		})
		.unwrap()
	}
}

#[derive(Debug)]
struct InvalidOp;

/// A structure used for handling IPC.
pub struct IPC {
	/// The address of the transmit queue.
	transmit_queue: NonNull<Packet>,
	/// The index of the next entry to be processed in the transmit queue.
	transmit_index: RingIndex,
	/// The address of the receive queue.
	receive_queue: NonNull<Packet>,
	/// The index of the next entry to be processed in the receive queue.
	receive_index: RingIndex,
	/// A list of address that can be freely mapped for IPC.
	free_pages: NonNull<FreePage>,
	/// The maximum amount of free pages.
	max_free_pages: usize,
}

impl IPC {
	/// Create a new IPC structure.
	pub fn new(
		transmit_queue: NonNull<Packet>,
		transmit_queue_bits: u8,
		receive_queue: NonNull<Packet>,
		receive_queue_bits: u8,
		free_pages: NonNull<FreePage>,
		free_pages_size: usize,
	) -> Self {
		let transmit_index = RingIndex::new(transmit_queue_bits);
		let receive_index = RingIndex::new(receive_queue_bits);
		let max_free_pages = free_pages_size;
		Self {
			transmit_queue,
			transmit_index,
			receive_queue,
			receive_index,
			free_pages,
			max_free_pages,
		}
	}

	/// Process IPC packets to be transmitted.
	pub fn process_packets(&mut self, slf_task: &super::Task) {
		use crate::arch::vms::VirtualMemorySystem;
		slf_task.inner().shared_state.virtual_memory.activate(); // TODO do this beforehand and once only
		arch::set_supervisor_userpage_access(true);
		let cqi = &mut self.transmit_index;
		loop {
			let cq = unsafe { &mut *self.transmit_queue.as_ptr().add(cqi.get()) };
			if let Some(op) = cq.opcode {
				let op = Op::try_from(op).unwrap();
				match op {
					Op::Read => {}
					Op::Write => {
						let bits = mem::size_of::<usize>() * 4;
						let (group, task) = (cq.address >> bits, cq.address & (1 << bits) - 1);
						let task = Group::get(group).unwrap().task(task).unwrap();
						cq.opcode = None;
						let pkt = cq.clone();
						drop(cq);
						// FIXME this is terribly inefficient
						task.inner().shared_state.virtual_memory.activate();
						let task_ipc = task.inner().ipc.as_mut().unwrap();
						let rxq = unsafe { &mut *task_ipc.receive_queue.as_ptr().add(0) };
						let addr = task_ipc.pop_free_range(1 /* FIXME */).expect("no free ranges");
						rxq.opcode = Some(op.into());
						rxq.length = pkt.length;
						unsafe { rxq.data.raw = addr.as_ptr() };
						// FIXME ditto
						slf_task.inner().shared_state.virtual_memory.activate();
						task.inner()
							.shared_state
							.virtual_memory
							.share(
								addr,
								unsafe { Page::from_pointer(pkt.data.raw).unwrap() },
								arch::vms::RWX::R,
								arch::vms::Accessibility::UserLocal,
							)
							.unwrap();
					}
				}
			} else {
				break;
			}
			cqi.increment();
		}
		arch::set_supervisor_userpage_access(false);
	}

	/// Pop an address range from the free ranges list.
	fn pop_free_range(&mut self, size: usize) -> Option<Page> {
		let free_pages =
			unsafe { core::slice::from_raw_parts_mut(self.free_pages.as_ptr(), self.max_free_pages) };
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
}

/// A single free page.
#[repr(C)]
pub struct FreePage {
	address: Option<NonNull<()>>,
	count: usize,
}

impl super::Task {
	/// Process IPC packets to be transmitted.
	pub fn process_io(&self) {
		self.inner().ipc.as_mut().map(|ipc| ipc.process_packets(self));
	}
}
