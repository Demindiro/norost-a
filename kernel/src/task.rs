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
use crate::memory::AllocatorError;
use crate::MEMORY_MANAGER;
use core::ptr::NonNull;

/// The fixed amount of page mappings. Some are mapped inside the `Task` structure itself to
/// improve the space usage of small tasks.
///
/// This is set to 16
const FIXED_PAGE_MAP_SIZE: usize = 16;

/// The order (i.e. size) of each `Task` structure.
const TASK_PAGE_ORDER: usize = 1;

/// A single task.
///
/// Tasks are implemented as a circular linked list for a few reasons:
/// - Removing the _current_ task can be done efficiently.
/// - When one task is done running, you switch to the task immediately after (it doesn't need to
///   be more complicated).
/// - It is very easy to make it space-efficient if the number of tasks is low.
/// - It goes on and on and on ... because it is circular. No need to go back to a "starting"
///   pointer or whatever.
struct Task {
	/// A pointer to the next task. It points to itself if there is only one task.
	next_task: NonNull<Task>,
	/// A pointer to the previous task or itself, which is needed to efficiently remove tasks.
	prev_task: NonNull<Task>,
	/// The ID of this task.
	id: u32,
	/// A fixed array of the start addresses of allocated memory pages.
	///
	/// A value of `None` means there is no entry **and** that it is the last entry.
	page_addresses: [Option<NonNull<Page>>; FIXED_PAGE_MAP_SIZE],
	/// The order (i.e. size) of each page map.
	page_orders: [u8; FIXED_PAGE_MAP_SIZE],
	/// A pointer to another page with additional page mappings. May be `None` if there are no
	/// extra mappings.
	extra_pages: Option<NonNull<Page>>,
	/// A structure to hold register state. Needed for context switches.
	register_state: arch::RegisterState,
}

/// An executor.
struct Executor {
	/// The list of tasks assigned to this executor. May be `None` if there are no tasks.
	tasks: Option<NonNull<Task>>,
}

struct NoTasks;

impl Task {
	/// Create a new empty task
	fn new() -> Result<Self, AllocatorError> {
		let len = arch::PAGE_SIZE << TASK_PAGE_ORDER;
		let pages = MEMORY_MANAGER.lock().allocate(TASK_PAGE_ORDER)?;
		pages.cast::<Task>().as_ptr().write(Task {
			next_task: pages.cast(),
			prev_task: pages.cast(),
			id: 0,
			page_addresses: [None; FIXED_PAGE_MAP_SIZE],
			page_orders: [0; FIXED_PAGE_MAP_SIZE],
			extra_pages: None,
		});
	}
}

impl Executor {
	/// Begins running the executor. This will not return.
	fn run() -> ! {
		loop {
			if let Some(curr) = self.tasks {
				
			} else {
				crate::powerstate::halt();
			}
		}
	}

	/// Begin executing the next task.
	fn next() {
		
	}

	/// Insert a new task right after the current one.
	///
	/// ## Safety
	///
	/// The task must be valid and not already in use by other executors.
	unsafe fn insert(&mut self, task: NonNull<Task>) {
		if let Some(curr) = self.tasks {
			let next = curr.next_task;
			task.prev_task = curr;
			task.next_task = next;
			curr.next_task = task;
			next.prev_task = task;
		} else {
			self.tasks = Some(task);
		}
	}

	/// Destroy the current task
	fn destroy(&mut self) -> Result<(), NoTasks> {
		if let Some(t) = self.tasks {
			if t == t.next_task {
				self.tasks = None;
			} else {
				t.prev_task.next_task = t.next_task;
				t.next_task.prev_task = t.prev_task;
			}
			// FIXME free it
			Ok(())
		} else {
			Err(NoTasks)
		}
	}
}
