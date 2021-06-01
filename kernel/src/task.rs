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

use crate::arch;
use crate::memory::AllocateError;
use crate::MEMORY_MANAGER;
use core::sync::atomic;
use core::ptr::NonNull;

/// The fixed amount of page mappings. Some are mapped inside the `Task` structure itself to
/// improve the space usage of small tasks.
///
/// This is set to 16
const FIXED_PAGE_MAP_SIZE: usize = 16;

/// The order (i.e. size) of each `Task` structure.
const TASK_PAGE_ORDER: u8 = 1;

/// A global counter for assigning Task IDs.
// TODO handle wrap around + try to keep TIDs low
static TASK_ID_COUNTER: atomic::AtomicU32 = atomic::AtomicU32::new(0);

/// A wrapper around a task pointer.
#[derive(Clone)]
#[repr(transparent)]
pub struct Task(NonNull<TaskData>);

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
	stack: NonNull<arch::Page>,
	/// A pointer to the next task. It points to itself if there is only one task.
	next_task: Task,
	/// A pointer to the previous task or itself, which is needed to efficiently remove tasks.
	prev_task: Task,
	/// The ID of this task.
	id: u32,
	/// A fixed array of the start addresses of allocated memory pages.
	///
	/// A value of `None` means there is no entry **and** that it is the last entry.
	page_addresses: [Option<NonNull<arch::Page>>; FIXED_PAGE_MAP_SIZE],
	/// The order (i.e. size) of each page map.
	page_orders: [u8; FIXED_PAGE_MAP_SIZE],
	/// A pointer to another page with additional page mappings. May be `None` if there are no
	/// extra mappings.
	extra_pages: Option<NonNull<arch::Page>>,
}

struct NoTasks;

impl Task {
	/// Create a new empty task.
	pub fn new() -> Result<Self, AllocateError> {
		let len = arch::PAGE_SIZE << TASK_PAGE_ORDER;
		let pages = MEMORY_MANAGER.lock().allocate(TASK_PAGE_ORDER)?;
		let stack = MEMORY_MANAGER.lock().allocate(0)?.start();
		let task_data = pages.start().cast::<TaskData>();
		let task = Self(task_data);
		// SAFETY: task is valid
		unsafe {
			task_data.as_ptr().write(TaskData {
				register_state: Default::default(),
				stack,
				next_task: task.clone(),
				prev_task: task.clone(),
				id: TASK_ID_COUNTER.fetch_add(1, atomic::Ordering::Relaxed),
				page_addresses: [None; FIXED_PAGE_MAP_SIZE],
				page_orders: [0; FIXED_PAGE_MAP_SIZE],
				extra_pages: None,
			});
		}
		Ok(task)
	}

	/// Add a memory mapping to this task.
	pub fn add_mapping(&self, page: NonNull<arch::Page>, order: u8) -> Result<(), AllocateError> {
		// FIXME
		self.inner().page_addresses[0] = Some(page);
		self.inner().page_orders[0] = order;
		Ok(())
	}

	/// Set the program counter of this task to the given address.
	pub fn set_pc(&self, address: *const ()) {
		self.inner().register_state.set_pc(address);
	}

	/// Begin executing the next task.
	pub fn next(self) -> ! {
		let task = self.inner().next_task.clone();
		crate::log::debug_str("NEXT");
		crate::log::debug_usize("tid", task.id() as usize, 10);
		crate::log::debug_usize("pc", task.inner().register_state.pc as usize, 16);
		// SAFETY: even if the task invokes UB, it won't affect the kernel itself.
		unsafe { arch::trap_start_task(task) }
	}

	/// Insert a new task right after the current one. This removes it from any other task lists.
	pub fn insert(&self, mut task: Task) {
		unsafe {
			// Remove from current list
			task.inner().prev_task.inner().next_task = task.inner().next_task.clone();
			task.inner().next_task.inner().prev_task = task.inner().prev_task.clone();
			// Insert in new list
			let prev = self.inner().prev_task.clone();
			let next = self.inner().next_task.clone();
			task.inner().prev_task = prev.clone();
			task.inner().next_task = next.clone();
			prev.inner().next_task = task.clone();
			next.inner().prev_task = task;
		}
	}

	/// Return the ID of this task
	pub fn id(&self) -> u32 {
		self.inner().id
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
	exec.next()
}

/*
impl Executor {
	/// Destroy this task, removing it from the list it is part of.
	fn destroy(&mut self) -> Result<(), NoTasks> {
		if let Some(mut t) = self.tasks {
			// SAFETY: the task is valid.
			let tr = unsafe { t.as_mut() };
			if t == tr.next_task {
				self.tasks = None;
			} else {
				// SAFETY: the tasks are valid and they don't alias `tr`
				// next_task and prev_task may alias each other, but with
				// scoping it is not an issue (i.e. there are no two mutable
				// references to the same struct simultaneously).
				unsafe { tr.prev_task.as_mut().next_task = tr.next_task };
				unsafe { tr.next_task.as_mut().prev_task = tr.prev_task };
			}
			// FIXME free it
			Ok(())
		} else {
			Err(NoTasks)
		}
	}
}
*/
