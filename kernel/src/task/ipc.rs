//! Structures & functions to facilitate inter-task communication

use crate::arch::{self, Page};
use core::mem;
use core::num::NonZeroU8;

/// Structure representing an index and mask in a ring buffer.
pub struct RingIndex {
	mask: u16,
	index: u16,
}

impl RingIndex {
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
union Data {
	/// An address range with raw data to be read or to which data should be written.
	raw: *mut u8,
}

/// An IPC packet.
#[repr(C)]
pub struct Packet {
	opcode: Option<NonZeroU8>,
	priority: i8,
	flags: u16,
	id: u32,
	address: usize,
	length: usize,
	data: Data,
}

impl super::Task {
	/// Process I/O entries and begin executing the next task.
	pub fn process_io(&self) {
		if let Some(cq) = self.inner().client_request_queue {
			use crate::arch::vms::VirtualMemorySystem;
			self.inner().shared_state.virtual_memory.activate(); // TODO do this beforehand and once only
			arch::set_supervisor_userpage_access(true);
			let mut cq = cq.cast::<[Packet; Page::SIZE / mem::size_of::<Packet>()]>();
			let cq = unsafe { cq.as_mut() };
			let cqi = &mut self.inner().client_request_index;
			loop {
				let cq = &mut cq[cqi.get()];
				if let Some(_op) = cq.opcode {
					// Just assume write for now.
					let s = unsafe { core::slice::from_raw_parts(cq.data.raw, cq.length) };
					let s = unsafe { core::str::from_utf8_unchecked(s) };
					use core::fmt::Write;
					write!(crate::log::Log, "{}", s).unwrap();
					cq.opcode = None;
				} else {
					break;
				}
				cqi.increment();
			}
			arch::set_supervisor_userpage_access(false);
		}
	}
}
