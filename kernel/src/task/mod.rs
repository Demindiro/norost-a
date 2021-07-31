//! Module for managing tasks (i.e. threads, processes ...)
//!
//! There are two structures of note:
//!
//! - [`Task`](Task) holds register state and **physical** memory mappings.
//! - [`Executor`] runs tasks and handles traps. Generally there is only one executor
//!   per hart. Each executor has a single task list and may request to swap tasks out
//!   if the load is high.
//!
//! It should be noted that all kernel tasks use physical addresses directly. Virtual addresses
//! are managed by these kernel tasks instead. Some protection can be offered depending on the
//! architecture (e.g. RISC-V implementations may have `PMP`, which offers restrictions similar
//! to that offered by virtual memory).
//!
//! Normally, the tasks run in a lower privilege mode than the kernel. This does depend on hardware
//! support though: if there are only one or two privilege levels the kernel tasks will run at the
//! highest level. This does sacrifice some security but there is not much that can be done about
//! it.

mod executor;
mod group;
mod ipc;

pub use executor::Executor;
pub use group::Group;

use ipc::RingIndex;

use crate::arch::vms::{self, VirtualMemorySystem, RWX};
use crate::arch::{self, Map, Page};
use crate::memory::{self, AllocateError};
use core::ptr::NonNull;
use core::sync::atomic;

/// A wrapper around a task pointer.
#[derive(Clone)]
#[repr(C)]
pub struct Task(NonNull<TaskData>);

/// State that can be shared between multiple tasks.
#[repr(C)]
struct SharedState {
	/// Mapping of virtual memory.
	virtual_memory: arch::VMS,
}

/// A single task.
///
/// Tasks are implemented as a circular linked list for a few reasons:
/// - Removing the _current_ task can be done efficiently.
/// - When one task is done running, you switch to the task immediately after (it doesn't need to
///   be more complicated).
/// - It is very easy to make it space-efficient if the number of tasks is low.
/// - It goes on and on and on ... because it is circular. No need to go back to a "starting"
///   pointer or whatever.
#[repr(C)]
pub struct TaskData {
	/// The register state of this task. Needed for context switches.
	register_state: arch::RegisterState,
	/// A pointer to some stack space for use with syscalls.
	stack: Page,
	/// The shared state of this task.
	// TODO should be reference counted.
	shared_state: SharedState,
	/// The address of the transmit queue.
	transmit_queue: Option<NonNull<Page>>,
	/// The address of the receive queue.
	receive_queue: Option<NonNull<Page>>,
	/// The index of the next entry to be processed in the transmit queue.
	transmit_index: RingIndex,
	/// The index of the next entry to be processed in the receive queue.
	receive_index: RingIndex,
}

const STACK_ADDRESS: Page = memory::reserved::HART_STACKS.start;
static mut TASK_DATA_ADDRESS: Page = memory::reserved::TASK_DATA.start;

impl Task {
	/// Create a new empty task with the given VMS.
	pub fn new(vms: arch::VMS) -> Result<Self, AllocateError> {
		// FIXME may leak memory on alloc error.
		let task_data = Map::Private(memory::allocate()?);
		unsafe {
			vms.add_to(
				TASK_DATA_ADDRESS,
				task_data,
				RWX::RW,
				vms::Accessibility::KernelGlobal,
			)
			.unwrap();
		}
		let task = Self(unsafe { TASK_DATA_ADDRESS }.as_non_null_ptr());
		// SAFETY: task is valid
		unsafe {
			task.0.as_ptr().write(TaskData {
				register_state: Default::default(),
				stack: STACK_ADDRESS.next().unwrap(),
				shared_state: SharedState {
					virtual_memory: vms,
				},
				transmit_queue: None,
				receive_queue: None,
				transmit_index: RingIndex::default(),
				receive_index: RingIndex::default(),
			});
		}
		unsafe { TASK_DATA_ADDRESS = TASK_DATA_ADDRESS.next().unwrap() };
		Ok(task)
	}

	/// Set the program counter of this task to the given address.
	pub fn set_pc(&self, address: *const ()) {
		self.inner().register_state.set_pc(address);
	}

	/// Begin executing this task.
	fn execute(&self) -> ! {
		self.inner().shared_state.virtual_memory.activate();
		// SAFETY: even if the task invokes UB, it won't affect the kernel itself.
		unsafe { arch::trap_start_task(self.clone()) };
	}

	/// Allocate private memory at the given virtual address.
	pub fn allocate_memory(
		&self,
		address: Page,
		count: usize,
		rwx: vms::RWX,
	) -> Result<(), vms::AddError> {
		self.inner().shared_state.virtual_memory.allocate(
			address,
			count,
			rwx,
			vms::Accessibility::UserLocal,
		)
	}

	/// Deallocate memory
	pub fn deallocate_memory(&self, address: NonNull<arch::Page>, count: usize) -> Result<(), ()> {
		let _ = (address, count);
		todo!()
		//self.inner().shared_state.virtual_memory.deallocate(address, count)
	}

	/// Set the task transmit & receive queue pointers and sizes.
	pub fn set_queues(&self, buffers: Option<((NonNull<Page>, u8), (NonNull<Page>, u8))>) {
		if let Some(((txq, txs), (rxq, rxs))) = buffers {
			self.inner().transmit_index.set_mask(txs);
			self.inner().receive_index.set_mask(rxs);
			self.inner().transmit_queue = Some(txq);
			self.inner().receive_queue = Some(rxq);
		} else {
			self.inner().transmit_queue = None;
			self.inner().receive_queue = None;
		}
	}

	fn inner<'a>(&'a self) -> &'a mut TaskData {
		// SAFETY: The task has been safely initialized.
		unsafe { self.0.clone().as_mut() }
	}
}

/// Begin executing the next task.
// TODO figure out how to get this to work in `impl`s
#[export_name = "executor_next_task"]
#[linkage = "external"]
extern "C" fn next(exec: Task) -> ! {
	exec.process_io();
	exec.execute()
}
