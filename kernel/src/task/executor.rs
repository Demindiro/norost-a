//! # Executor
//!
//! An executor schedules & runs tasks. Normally, there is exactly one executor per hart.

use super::*;
use crate::arch;
use crate::task::Task;
use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::Ordering;

/// The idle "task".
///
/// This is not a real task. It is simply used as a buffer so trap handlers don't need yet
/// another branch to ensure no memory is improperly overwritten.
///
/// While writing to data from multiple harts without synchronization may technically be
/// UB, it is unlikely to be an issue since we normally don't read from the written data.
static IDLE_TASK_STUB: WriteOnly<UnsafeCell<MaybeUninit<TaskData>>> =
	WriteOnly(UnsafeCell::new(MaybeUninit::uninit()));

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
// FIXME this really needs to be fixed, it just got out of sync due to an interrupt which caused
// some very strange buggy behaviour.
static mut NEXT_ID: usize = 0;

impl Executor<'_> {
	/// Suspend the current task (if any) and begin executing another task.
	pub fn next() -> ! {
		// Unclaim the current task
		Self::current_task()
			.inner()
			.executor_id
			.store(u16::MAX, Ordering::Relaxed);

		// TODO lol, lmao

		let group = group::Group::get(0).expect("No root group");

		let prev_id = unsafe { NEXT_ID };
		// Incrementing by prime numbers because I'm a genius hacker hmmm yes yes
		let mut id = (prev_id + 7) & 0xf;

		let mut min_time = u64::MAX;
		let curr_time = arch::current_time();
		let mut stop_next = false;

		loop {
			if let Ok(task) = group.task(id) {
				let wait_time = task.inner().wait_time;
				if wait_time < curr_time {
					unsafe { NEXT_ID = id };
					arch::schedule_timer(10_000_000 / 10);
					// If the task is already claimed, just try again.
					arch::enable_interrupts(true);
					let _ = task.execute(Self::id());
				}
				min_time = min_time.min(wait_time);
			};
			id = id.wrapping_add(7) & 0xf;
			if id == prev_id {
				stop_next = true;
			} else if stop_next {
				break;
			}
		}

		Self::idle(min_time)
	}

	/// Returns the address of the current task
	pub fn current_address() -> Address {
		// FIXME
		Address::todo(unsafe { NEXT_ID })
	}

	/// Begin idling, i.e. do nothing until the given time.
	#[allow(dead_code)]
	pub fn idle(time: u64) -> ! {
		unsafe {
			// TODO move this to arch::
			asm!("csrw sscratch, {0}", in(reg) IDLE_TASK_STUB.0.get());
		}
		arch::set_timer(time);
		arch::enable_kernel_interrupts(true);
		loop {
			crate::powerstate::halt();
		}
	}

	/// Initializes the executor for a given hart.
	///
	/// # Safety
	///
	/// It must only be called once per hart and only by the hart that will use
	/// this executor.
	///
	/// # Panics
	///
	/// If it failed to allocate memory or if the stack address is out of range.
	pub fn init(id: u16) {
		const STACK_ADDRESS: Page = crate::memory::reserved::HART_STACKS.start;

		// FIXME HACK
		unsafe {
			(&mut *(&mut *IDLE_TASK_STUB.0.get()).as_mut_ptr()).stack =
				crate::memory::reserved::HART_STACKS.start.skip(1).unwrap()
		};
		unsafe {
			(&mut *(&mut *IDLE_TASK_STUB.0.get()).as_mut_ptr())
				.executor_id
				.store(id, Ordering::Relaxed)
		};

		// TODO should be moved to arch::
		unsafe { asm!("csrw sscratch, {0}", in(reg) IDLE_TASK_STUB.0.get()) };

		let stack = Map::Private(memory::allocate().unwrap());
		arch::VMS::add(
			STACK_ADDRESS,
			stack,
			RWX::RW,
			vms::Accessibility::KernelGlobal,
		)
		.unwrap();
	}

	/// Return the ID of this executor, which corresponds to the hart ID.
	pub fn id() -> u16 {
		Self::current_task()
			.inner()
			.executor_id
			.load(Ordering::Relaxed)
	}

	/// Return the current task claimed by this executor.
	pub fn current_task() -> Task {
		// TODO should be moved partially to arch::
		let task: *mut TaskData;
		unsafe { asm!("csrr {0}, sscratch", out(reg) task) };
		// SAFETY: sscratch should never ever EVER be 0! Hence it _should_ be safe to
		// make it NonNull
		unsafe {
			Task {
				ptr: NonNull::new_unchecked(task),
			}
		}
	}
}

impl super::Task {
	/// Delay the task for the given duration
	pub fn wait_duration(&self, delay: u64) {
		// Clamp the duration in case delay is very high.
		self.inner().wait_time = arch::current_time().checked_add(delay).unwrap_or(u64::MAX);
	}
}

/// Helper function primarily intended to be called from assembly.
#[export_name = "executor_get_task"]
extern "C" fn get_task(address: Address) -> Option<Task> {
	// FIXME *puke*
	unsafe { NEXT_ID = address.into() };
	group::Group::get(address.group().into()).and_then(|g| g.task(address.task().into()).ok())
}

/// Helper function primarily intended to be called from assembly.
#[export_name = "executor_next_task"]
extern "C" fn next_task() -> ! {
	Executor::next()
}
