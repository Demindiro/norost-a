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

use crate::arch::{self, Page, PAGE_SIZE};
use crate::memory::{self, AllocateError, PPN};
use core::mem;
use core::num::NonZeroU8;
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

/// State that can be shared between multiple tasks.
#[repr(C)]
struct SharedState {
	/// Mapping of virtual memory.
	virtual_memory: arch::VirtualMemorySystem,
}

/// Structure representing an index and mask in a ring buffer.
struct RingIndex {
	mask: u16,
	index: u16,
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
	stack: NonNull<arch::Page>,
	/// The shared state of this task.
	// TODO should be reference counted.
	shared_state: SharedState,
	/// The virtual address of the client request buffer.
	client_request_queue: Option<NonNull<Page>>,
	/// The virtual address of the client completion buffer.
	client_completion_queue: Option<NonNull<Page>>,
	/// The virtual address of the server request buffer.
	server_request_queue: Option<NonNull<Page>>,
	/// The virtual address of the server completion buffer.
	server_completion_queue: Option<NonNull<Page>>,
	/// The index of the next entry to be processed in the client request buffer.
	client_request_index: RingIndex,
	/// The index of the next entry to be processed in the client completion buffer.
	client_completion_index: RingIndex,
	/// The index of the next entry to be processed in the server request buffer.
	server_request_index: RingIndex,
	/// The index of the next entry to be processed in the server completion buffer.
	server_completion_index: RingIndex,
	/// A pointer to the next task. It points to itself if there is only one task.
	next_task: Task,
	/// A pointer to the previous task or itself, which is needed to efficiently remove tasks.
	prev_task: Task,
	/// The ID of this task.
	id: u32,
}

union ClientRequestEntryData {
	pages: Option<NonNull<Page>>,
	name: *const u8,
	uuid: *const u8,
}

/// A client request entry.
#[repr(align(64))]
#[repr(C)]
struct ClientRequestEntry {
	opcode: Option<NonZeroU8>,
	priority: i8,
	flags: u16,
	file_handle: u32,
	offset: usize,
	data: ClientRequestEntryData,
	length: usize,
	userdata: usize,
}

union ClientCompletionEntryData {
	pages: Option<NonNull<Page>>,
	file_handle: usize,
}

/// A client completion entry.
#[repr(align(32))]
#[repr(C)]
struct ClientCompletionEntry {
	data: ClientCompletionEntryData,
	length: usize,
	status: u32,
	userdata: usize,
}

const _SIZE_CHECK_CRE: usize = 0 - (64 - mem::size_of::<ClientRequestEntry>());
const _SIZE_CHECK_CCE: usize = 0 - (32 - mem::size_of::<ClientCompletionEntry>());

struct NoTasks;

impl Default for RingIndex {
	fn default() -> Self {
		Self {
			mask: 0,
			index: 0,
		}
	}
}

impl RingIndex {
	#[inline(always)]
	fn set_mask(&mut self, mask: u8) {
		let mask = (1 << mask) - 1;
		self.index &= mask;
		self.mask = mask;
	}

	fn increment(&mut self) {
		self.index += 1;
		self.index &= self.mask;
	}
	
	fn get(&self) -> usize {
		self.index.into()
	}
}

impl Task {
	/// Create a new empty task.
	pub fn new() -> Result<Self, AllocateError> {
		// FIXME may leak memory on alloc error.
		let len = arch::PAGE_SIZE << TASK_PAGE_ORDER;
		let pages = memory::mem_allocate(TASK_PAGE_ORDER)?;
		let stack = memory::mem_allocate(0)?;
		let stack = usize::from(stack);
		let stack = stack << 12;
		let stack = stack as *mut _;
		let stack = NonNull::new(stack).unwrap();
		let task_data = pages;
		let task_data = usize::from(task_data);
		let task_data = task_data << 12;
		let task_data = task_data as *mut _;
		let task_data = NonNull::new(task_data).unwrap();
		let task = Self(task_data);
		// SAFETY: task is valid
		unsafe {
			task_data.as_ptr().write(TaskData {
				register_state: Default::default(),
				stack,
				next_task: task.clone(),
				prev_task: task.clone(),
				id: TASK_ID_COUNTER.fetch_add(1, atomic::Ordering::Relaxed),
				shared_state: SharedState {
					virtual_memory: arch::VirtualMemorySystem::new()?,
				},
				client_request_queue: None,
				client_completion_queue: None,
				server_request_queue: None,
				server_completion_queue: None,
				client_request_index: RingIndex::default(),
				client_completion_index: RingIndex::default(),
				server_request_index: RingIndex::default(),
				server_completion_index: RingIndex::default(),
			});
		}
		Ok(task)
	}

	/// Add a memory mapping to this task.
	pub fn add_mapping(&self, address: NonNull<Page>, page: PPN, rwx: arch::RWX) -> Result<(), crate::arch::riscv::vms::AddError> {
		self.inner().shared_state.virtual_memory.add(address, page, rwx)
	}

	/// Set the program counter of this task to the given address.
	pub fn set_pc(&self, address: *const ()) {
		self.inner().register_state.set_pc(address);
	}

	/// Process I/O entries and begin executing the next task.
	pub fn next(self) -> ! {
		let task = self.inner().next_task.clone();
		if let Some(cq) = task.inner().client_request_queue {
			let cqi = &mut task.inner().client_request_index;
			let (cq, rwx) = task.inner().shared_state.virtual_memory.get(cq.cast()).unwrap();
			assert!(rwx.r());
			let cq = NonNull::new((usize::from(cq) << 12) as *mut Page).unwrap();
			let mut cq = cq.cast::<[ClientRequestEntry; PAGE_SIZE / mem::size_of::<ClientRequestEntry>()]>();
			let cq = unsafe { cq.as_mut() };
			loop {
				let cq = &mut cq[cqi.get()];
				if let Some(op) = cq.opcode {
					// Just assume write for now.
					let ss = unsafe { cq.data.pages.unwrap() };
					let s = task.inner().shared_state.virtual_memory.get(ss).unwrap().0;
					let s = NonNull::new(((usize::from(s) << 12) + ss.as_ptr().align_offset(4096)) as *mut u8).unwrap().as_ptr();
					let s = unsafe { core::slice::from_raw_parts(s, cq.length) };
					let s = unsafe { core::str::from_utf8_unchecked(s) };
					log!("{}", s);
					cq.opcode = None;
				} else {
					break;
				}
				cqi.increment();
			}
		}
		let pc = task.inner().shared_state.virtual_memory.get(NonNull::new(task.inner().register_state.pc as *mut _).unwrap());
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

	/// Return the physical address the given virtual address maps to.
	pub fn translate_virtual_address(&self, address: NonNull<Page>) -> Option<(PPN, crate::arch::RWX)> {
		self.inner().shared_state.virtual_memory.get(address)
	}

	/// Allocate private memory at the given virtual address.
	pub fn allocate_memory(&self, address: NonNull<crate::arch::Page>, count: usize, rwx: crate::arch::RWX) -> Result<(), crate::arch::riscv::vms::AddError> {
		self.inner().shared_state.virtual_memory.allocate(address, count, rwx)
	}

	/// Allocate shared memory at the given virtual address.
	pub fn allocate_shared_memory(&self, address: NonNull<crate::arch::Page>, count: usize, rwx: crate::arch::RWX) -> Result<(), ()> {
		self.inner().shared_state.virtual_memory.allocate_shared(address, count, rwx)
	}

	/// Deallocate memory
	pub fn deallocate_memory(&self, address: NonNull<crate::arch::Page>, count: usize) -> Result<(), ()> {
		self.inner().shared_state.virtual_memory.deallocate(address, count)
	}

	/// Return the ID of this task
	pub fn id(&self) -> u32 {
		self.inner().id
	}

	/// Set the task client request and completion buffer pointers and sizes.
	pub fn set_client_buffers(&self, buffers: Option<((NonNull<Page>, u8), (NonNull<Page>, u8))>) {
		if let Some(((rb, rbs), (cb, cbs))) = buffers {
			self.inner().client_request_index.set_mask(rbs);
			self.inner().client_completion_index.set_mask(cbs);
			self.inner().client_request_queue = Some(rb);
			self.inner().client_completion_queue = Some(cb);
		} else {
			self.inner().client_request_queue = None;
			self.inner().client_completion_queue = None;
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
