//! # Executor
//!
//! An executor schedules & runs tasks. Normally, there is exactly one executor per hart.

use super::*;
use crate::arch;

#[repr(C)]
pub struct Executor<'a> {
	/// A pointer to the current task being executed.
	current_task: Option<group::Guard<'a>>,
}

#[derive(Debug)]
pub struct NoTask;

// FIXME lol wtf
static mut NEXT_ID: usize = 0;

impl Executor<'_> {
	/// Suspend the current task (if any) and begin executing another task.
	pub fn next(&self) -> ! {
		// TODO lol, lmao
		let id = unsafe { NEXT_ID };
		let task = group::Group::get(0)
			.expect("No root group")
			.task(id)
			.expect("No task 0");
		if id > 0 {
			unsafe { NEXT_ID = 0 };
		} else {
			unsafe { NEXT_ID = id + 1 };
		}
		task.execute()
	}

	/// Process I/O of the current task
	pub fn process_io(&self) -> Result<(), NoTask> {
		let task = self.current_task.as_ref().ok_or(NoTask)?;
		if let Some(cq) = task.inner().client_request_queue {
			arch::set_supervisor_userpage_access(true);
			let mut cq =
				cq.cast::<[ClientRequestEntry; PAGE_SIZE / mem::size_of::<ClientRequestEntry>()]>();
			let cq = unsafe { cq.as_mut() };
			let cqi = &mut task.inner().client_request_index;
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
		Ok(())
	}

	/// Begin idling, i.e. do nothing
	pub fn idle(&self) -> ! {
		loop {
			crate::powerstate::halt();
		}
	}
}

impl Default for Executor<'_> {
	fn default() -> Self {
		Self { current_task: None }
	}
}
