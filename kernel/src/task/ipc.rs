//! Structures & functions to facilitate inter-task communication

use super::group::Group;
use crate::arch::{self, Page};
use core::convert::TryFrom;
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

#[derive(Debug)]
struct InvalidOp;

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

impl super::Task {
	/// Process I/O entries and begin executing the next task.
	pub fn process_io(&self) {
		if let Some(cq) = self.inner().transmit_queue {
			use crate::arch::vms::VirtualMemorySystem;
			self.inner().shared_state.virtual_memory.activate(); // TODO do this beforehand and once only
			arch::set_supervisor_userpage_access(true);
			let mut cq = cq.cast::<[Packet; Page::SIZE / mem::size_of::<Packet>()]>();
			let cq = unsafe { cq.as_mut() };
			let cqi = &mut self.inner().transmit_index;
			loop {
				let cq = &mut cq[cqi.get()];
				if let Some(op) = cq.opcode {
					let op = Op::try_from(op).unwrap();
					match op {
						Op::Read => {}
						Op::Write => {
							use core::fmt::Write;
							if cq.address == usize::MAX {
								// FIXME this is a temporary workaround for not having a "video" driver task
								let s =
									unsafe { core::slice::from_raw_parts(cq.data.raw, cq.length) };
								let s = unsafe { core::str::from_utf8_unchecked(s) };
								write!(crate::log::Log, "{}", s).unwrap();
								cq.opcode = None;
							} else {
								writeln!(crate::log::Log, "Sending packet to {}", cq.address);
								let bits = mem::size_of::<usize>() * 4;
								let (group, task) =
									(cq.address >> bits, cq.address & (1 << bits) - 1);
								let task = Group::get(group).unwrap().task(task).unwrap();
								let pkt = cq.clone();
								drop(cq);
								// FIXME this is terribly inefficient
								task.inner().shared_state.virtual_memory.activate();
								let rxq = task.inner().receive_queue.unwrap();
								let rxq = unsafe {
									rxq.cast::<[Packet; Page::SIZE / mem::size_of::<Packet>()]>()
										.as_mut()
								};
								unsafe {
									assert_ne!(rxq[0].data.raw, core::ptr::null_mut());
								}
								rxq[0].opcode = Some(op.into());
								rxq[0].length = pkt.length;
								let addr = unsafe { Page::from_pointer(rxq[0].data.raw).unwrap() };
								// FIXME ditto
								self.inner().shared_state.virtual_memory.activate();
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
					}
				} else {
					break;
				}
				cqi.increment();
			}
			arch::set_supervisor_userpage_access(false);
		}
	}
}
