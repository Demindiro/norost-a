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

pub use executor::Executor;
pub use group::Group;

use crate::arch::vms::{self, VirtualMemorySystem, RWX};
use crate::arch::{self, Map, Page};
use crate::memory::{self, AllocateError};
use core::mem;
use core::num::NonZeroU8;
use core::ptr::NonNull;
use core::sync::atomic;

/// A global counter for assigning Task IDs.
// TODO handle wrap around + try to keep TIDs low
static TASK_ID_COUNTER: atomic::AtomicU32 = atomic::AtomicU32::new(0);

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
	stack: Page,
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
	/// The ID of this task.
	id: u32,
}

union ClientRequestEntryData {
	pages: Option<NonNull<Page>>,
	#[allow(dead_code)]
	name: *const u8,
	#[allow(dead_code)]
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
	#[allow(dead_code)]
	pages: Option<NonNull<Page>>,
	#[allow(dead_code)]
	file_handle: usize,
}

/// A client completion entry.
#[repr(align(32))]
#[repr(C)]
#[allow(dead_code)]
struct ClientCompletionEntry {
	data: ClientCompletionEntryData,
	length: usize,
	status: u32,
	userdata: usize,
}

const _SIZE_CHECK_CRE: usize = 0 - (64 - mem::size_of::<ClientRequestEntry>());
const _SIZE_CHECK_CCE: usize = 0 - (32 - mem::size_of::<ClientCompletionEntry>());

impl Default for RingIndex {
	fn default() -> Self {
		Self { mask: 0, index: 0 }
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
				id: TASK_ID_COUNTER.fetch_add(1, atomic::Ordering::Relaxed),
				shared_state: SharedState {
					virtual_memory: vms,
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
		unsafe { TASK_DATA_ADDRESS = TASK_DATA_ADDRESS.next().unwrap() };
		Ok(task)
	}

	/// Set the program counter of this task to the given address.
	pub fn set_pc(&self, address: *const ()) {
		self.inner().register_state.set_pc(address);
	}

	/// Process I/O entries and begin executing the next task.
	pub fn process_io(&self) {
		if let Some(cq) = self.inner().client_request_queue {
			arch::set_supervisor_userpage_access(true);
			let mut cq = cq
				.cast::<[ClientRequestEntry; Page::SIZE / mem::size_of::<ClientRequestEntry>()]>();
			let cq = unsafe { cq.as_mut() };
			let cqi = &mut self.inner().client_request_index;
			loop {
				let cq = &mut cq[cqi.get()];
				if let Some(_op) = cq.opcode {
					// Just assume write for now.
					let s = unsafe { cq.data.pages.unwrap().cast() };
					let s = unsafe { core::slice::from_raw_parts(s.as_ptr(), cq.length) };
					let s = unsafe { core::str::from_utf8_unchecked(s) };
					log!("{}", s);
					cq.opcode = None;
				} else {
					break;
				}
				cqi.increment();
			}
			arch::set_supervisor_userpage_access(false);
		}
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
	exec.process_io();
	exec.execute()
}
