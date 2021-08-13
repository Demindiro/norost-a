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

pub mod ipc;
pub mod notification;

mod address;
mod executor;
mod group;

pub use address::*;
pub use executor::Executor;
pub use group::Group;

use crate::arch::vms::{self, VirtualMemorySystem, RWX};
use crate::arch::{self, Map, Page};
use crate::memory::{self, AllocateError};
use core::num::NonZeroU16;
use core::ptr::NonNull;

/// Various flags indicating a task's state.
#[repr(transparent)]
struct Flags(u16);

impl Flags {
}

/// An IRQ source / identifier
// TODO move this to arch::
#[repr(transparent)]
struct IRQ(NonZeroU16);

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
#[repr(C)]
pub struct TaskData {
	/// The register state of this task. Needed for context switches.
	register_state: arch::RegisterState,
	/// A pointer to some stack space for use with syscalls.
	stack: Page,
	/// The shared state of this task.
	// TODO should be reference counted.
	shared_state: SharedState,
	/// The address of a notification handler.
	notification_handler: Option<notification::Handler>,
	/// Flags pertaining to this task
	flags: Flags,
	/// The IRQ this task is currently handling, if any.
	///
	/// Only relevant for drivers.
	current_irq: Option<IRQ>,
	/// IPC state to communicate with other tasks.
	ipc: Option<ipc::IPC>,
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
		let task = Self(unsafe { TASK_DATA_ADDRESS }.as_non_null_ptr().cast());
		// SAFETY: task is valid
		unsafe {
			task.0.as_ptr().write(TaskData {
				register_state: Default::default(),
				stack: STACK_ADDRESS.next().unwrap(),
				shared_state: SharedState {
					virtual_memory: vms,
				},
				notification_handler: None,
				current_irq: None,
				flags: Flags(0),
				ipc: None,
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

	/// Allocate private memory at the given virtual address for the current task.
	pub fn allocate_memory(
		address: Page,
		count: usize,
		rwx: vms::RWX,
	) -> Result<(), vms::AddError> {
		//self.inner().shared_state.virtual_memory
		arch::VMS::allocate(
			address,
			count,
			rwx,
			vms::Accessibility::UserLocal,
		)
	}

	/// Deallocate memory for the current task
	pub fn deallocate_memory(address: Page, count: usize) -> Result<(), ()> {
		let _ = (address, count);
		/*
		self.inner()
			.shared_state
			.virtual_memory
			*/
		arch::VMS::deallocate(address, count)
	}

	/// Set the task transmit & receive queue pointers and sizes.
	pub fn set_queues(&self, buffers: Option<ipc::IPC>) {
		self.inner().ipc = buffers;
	}

	fn inner<'a>(&'a self) -> &'a mut TaskData {
		// SAFETY: The task has been safely initialized.
		unsafe { self.0.clone().as_mut() }
	}
}
