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
pub mod registry;

mod executor;

pub use executor::Executor;

use crate::allocator::arena::{self, Arena};
use crate::arch::vms::{self, VirtualMemorySystem, RWX};
use crate::arch::{self, Map, Page};
use crate::memory::{self, reserved, AllocateError};
use core::cell::Cell;
use core::convert::TryFrom;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicPtr, AtomicU16, AtomicU32, AtomicU64, Ordering};

#[derive(Debug)]
struct Claimed(u16);

/// Various flags indicating a task's state.
///
/// A 32-bit atomic is used because 16/8-bit atomics result in terrible code generation for RISC-V
/// platforms.
#[derive(Default)]
#[repr(transparent)]
struct Flags(AtomicU32);

impl Flags {
	#[allow(dead_code)]
	const NOTIFYING: u32 = 0x1;
	const NOTIFIED: u32 = 0x2;
	const IPC_LOCK_TRANSMIT: u32 = 0x10;
	const IPC_LOCK_RECEIVED: u32 = 0x20;
	const DEAD: u32 = 0x8000;

	fn is_set(&self, bits: u32) -> bool {
		self.0.load(Ordering::Relaxed) & bits != 0
	}

	fn lock(&self, bits: u32) {
		let mut curr = self.0.load(Ordering::Relaxed);
		loop {
			if curr & bits != 0 {
				curr = self.0.load(Ordering::Relaxed);
				continue;
			}
			let new = curr | bits;
			match self
				.0
				.compare_exchange_weak(curr, new, Ordering::Acquire, Ordering::Relaxed)
			{
				Ok(_) => break,
				Err(c) => curr = c,
			}
		}
	}

	fn unlock(&self, bits: u32) {
		let mut curr = self.0.load(Ordering::Relaxed);
		loop {
			let new = curr & !bits;
			match self
				.0
				.compare_exchange_weak(curr, new, Ordering::Release, Ordering::Relaxed)
			{
				Ok(_) => break,
				Err(c) => curr = c,
			}
		}
	}

	fn set(&self, bits: u32) {
		let mut curr = self.0.load(Ordering::Relaxed);
		loop {
			let new = curr | bits;
			match self
				.0
				.compare_exchange_weak(curr, new, Ordering::Relaxed, Ordering::Relaxed)
			{
				Ok(_) => break,
				Err(c) => curr = c,
			}
		}
	}

	fn clear(&self, bits: u32) {
		let mut curr = self.0.load(Ordering::Relaxed);
		loop {
			let new = curr & !bits;
			match self
				.0
				.compare_exchange_weak(curr, new, Ordering::Relaxed, Ordering::Relaxed)
			{
				Ok(_) => break,
				Err(c) => curr = c,
			}
		}
	}
}

/// An IRQ source / identifier
///
/// Note that it is 32 bits wide: while atomic operations on sub-word size is possible for RISC-V,
/// it leads to much more verbose code. The increased size is considered worth it.
// TODO move this to arch::
#[derive(Default)]
#[repr(transparent)]
struct IRQ(AtomicU32);

/// A task ID, which is unique per task.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct TaskID(u32);

impl From<TaskID> for u32 {
	fn from(id: TaskID) -> Self {
		id.0
	}
}

impl From<u32> for TaskID {
	fn from(n: u32) -> Self {
		Self(n)
	}
}

impl From<TaskID> for usize {
	fn from(id: TaskID) -> Self {
		usize::try_from(id.0).unwrap()
	}
}

impl TryFrom<usize> for TaskID {
	type Error = <u32 as TryFrom<usize>>::Error;

	fn try_from(n: usize) -> Result<Self, Self::Error> {
		TryFrom::try_from(n).map(Self)
	}
}

/// A single task.
#[repr(C)]
pub struct Task {
	/// The register state of this task. Needed for context switches.
	register_state: arch::RegisterState,
	/// A pointer to some stack space for use with syscalls.
	stack: Page,
	/// Mapping of virtual memory.
	virtual_memory: arch::VMS,
	/// The address of a notification handler.
	notification_handler: AtomicPtr<u8>,
	/// The IRQ this task is currently handling, if any.
	///
	/// Only relevant for drivers.
	current_irq: IRQ,
	/// Flags pertaining to this task
	flags: Flags,
	/// The executor / hart that is executing this task.
	///
	/// This value is u16::MAX if no executor has claimed it.
	executor_id: AtomicU16,
	/// The time a task will wait for an event until it is rescheduled.
	wait_time: AtomicU64,
	/// IPC state to communicate with other tasks.
	ipc: ipc::IPC,
}

const STACK_ADDRESS: Page = memory::reserved::HART_STACKS.start;

/// The allocator for task data.
static TASKS: Arena<Task> = unsafe {
	Arena::new(
		reserved::TASKS.start.as_non_null_ptr().cast(),
		reserved::TASKS.byte_count(),
	)
};

/// The start of the destroy queue.
static DESTROY_QUEUE: Option<&'static Task> = None;

/// The start of the free list
static FREE_LIST: Option<&'static Task> = None;

pub type NewTaskError = arena::InsertError;

impl Task {
	/// Create a new empty task with the given VMS.
	pub fn new(
		vms: arch::VMS,
		program_counter: usize,
		stack_pointer: usize,
	) -> Result<TaskID, NewTaskError> {
		let id = TASKS.insert_with(|_| {
			let mut task = Task {
				register_state: Default::default(),
				stack: STACK_ADDRESS.next().unwrap(),
				virtual_memory: vms,
				notification_handler: Default::default(),
				current_irq: IRQ::default(),
				flags: Default::default(),
				executor_id: AtomicU16::new(u16::MAX),
				wait_time: Default::default(),
				ipc: Default::default(),
			};
			task.register_state.set_program_counter(program_counter);
			task.register_state.set_stack_pointer(stack_pointer);
			task
		})?;
		Ok(TaskID::try_from(id).unwrap())
	}

	/// Begin executing this task.
	///
	/// This function checks if the task is already claimed by another executor, hence it is safe
	/// to call at any time.
	fn execute(&self, executor_id: u16) -> Result<!, Claimed> {
		self.executor_id
			.compare_exchange(u16::MAX, executor_id, Ordering::Relaxed, Ordering::Relaxed)
			.map(|_| unsafe {
				self.virtual_memory.activate();
				arch::trap_start_task(&self)
			})
			.map_err(Claimed)
	}

	/// Allocate private memory at the given virtual address for the current task.
	pub fn allocate_memory(
		address: Page,
		count: usize,
		rwx: vms::RWX,
	) -> Result<(), vms::AddError> {
		arch::VMS::allocate(address, count, rwx, vms::Accessibility::UserLocal)
	}

	/// Deallocate memory for the current task
	pub fn deallocate_memory(address: Page, count: usize) -> Result<(), ()> {
		arch::VMS::deallocate(address, count)
	}

	/// Check if the task recently ran its notification handler.
	pub fn was_notified(&self) -> bool {
		self.flags.is_set(Flags::NOTIFIED)
	}

	/// Clear the notified flag.
	pub fn clear_notified(&self) {
		self.flags.clear(Flags::NOTIFIED)
	}

	/// Get the current IRQ being processed.
	pub fn current_irq(&self) -> u32 {
		self.current_irq.0.load(Ordering::Relaxed)
	}
}

unsafe impl Sync for Task {}

/// Return the alive task with the given ID if it exists and is alive.
fn get(id: TaskID) -> Option<&'static Task> {
	let id = usize::try_from(u32::from(id)).unwrap();
	TASKS
		.get(id)
		.and_then(|task| (!task.flags.is_set(Flags::DEAD)).then(|| task))
}
