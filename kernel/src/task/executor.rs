//! # Executor
//!
//! An executor schedules & runs tasks. Normally, there is exactly one executor per hart.

use super::*;
use crate::arch;
use crate::task::Task;
use core::cell::UnsafeCell;
use core::mem::MaybeUninit;

/// The idle "task".
///
/// This is not a real task. It is simply used as a buffer so trap handlers don't need yet
/// another branch to ensure no memory is improperly overwritten.
///
/// While writing to data from multiple harts without synchronization may technically be
/// UB, it is unlikely to be an issue since we normally don't read from the written data.
static IDLE_TASK_STUB: WriteOnly<UnsafeCell<MaybeUninit<Task>>> = WriteOnly(UnsafeCell::new(MaybeUninit::uninit()));

struct WriteOnly<T>(T);

unsafe impl<T> Sync for WriteOnly<T> {}

#[repr(C)]
pub struct Executor<'a> {
	/// The stack of this executor.
	stack: arch::Page,
	/// A pointer to the current task being executed.
	current_task: Option<group::Guard<'a>>,
}

#[derive(Debug)]
pub struct NoTask;

// FIXME lol wtf
// FIXME the boot elf parser doesn't zero initialize this. Workaround is to set it to 1 first (lmao)
static mut NEXT_ID: usize = 1;

impl Executor<'_> {
	/// Suspend the current task (if any) and begin executing another task.
	pub fn next(&self) -> ! {
		// TODO lol, lmao

		let group = group::Group::get(0).expect("No root group");

		loop {
			let id = unsafe { NEXT_ID };
			let id = if id > 256 {
				unsafe { NEXT_ID = 0 };
				0
			} else {
				unsafe { NEXT_ID = id + 1 };
				id + 1
			};

			if let Ok(task) = group.task(id) {
				task.process_io(Address::todo(id));
				task.execute()
			};
		}
	}

	/// Returns the address of the current task
	pub fn current_address() -> Address {
		// FIXME
		Address::todo(unsafe { NEXT_ID })
	}

	/// Begin idling, i.e. do nothing
	#[allow(dead_code)]
	pub fn idle() -> ! {
		//dbg!("Idling...");
		unsafe {
			// TODO move this to arch::
			asm!("csrw sscratch, {0}", in(reg) IDLE_TASK_STUB.0.get());
		}
		arch::enable_interrupts(true);
		loop {
			crate::powerstate::halt();
		}
	}

	/// Create a new executor.
	///
	/// # Panics
	///
	/// If it failed to allocate memory or if the stack address is out of range.
	pub fn new(id: usize) -> Self {
		const STACK_ADDRESS: Page = crate::memory::reserved::HART_STACKS.start;

		let stack = Map::Private(memory::allocate().unwrap());
		arch::VMS::add(
			STACK_ADDRESS,
			stack,
			RWX::RW,
			vms::Accessibility::KernelGlobal,
		)
		.unwrap();
		Self {
			stack: crate::memory::reserved::HART_STACKS
				.start
				.skip(id + 1)
				.unwrap(),
			current_task: None,
		}
	}

	// FIXME lol
	pub fn default() -> Self {
		Self {
			stack: crate::memory::reserved::HART_STACKS.start.skip(1).unwrap(),
			current_task: None,
		}
	}
}

/// Helper function primarily intended to be called from assembly.
#[export_name = "executor_get_task"]
extern "C" fn get_task(address: Address) -> Option<Task> {
	group::Group::get(address.group().into())
		.and_then(|g| g.task(address.task().into()).ok())
}
