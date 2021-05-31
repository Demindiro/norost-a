//! Machine-mode interrupt handler.
//!
//! This module contains generic code. Arch-specific code is located in [`arch`](crate::arch)

use crate::{arch, task};
use core::convert::TryFrom;

/// The type of a syscall, specifically the amount and type of arguments it takes.
pub type Syscall = extern "C" fn(a0: usize, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize, task: task::Task) -> Return;

/// The FFI-safe return value of syscalls
///
/// - The first field is the status of the call (i.e. did it succeed or was there an error).
/// - The second field is optional extra data that may be attached, such as a file descriptor.
#[repr(C)]
pub struct Return(Status, usize);

/// The length of the table as a separate constant because Rust is a little dum dum.
pub const TABLE_LEN: usize = 8;

/// Table with all syscalls.
#[export_name = "syscall_table"]
pub static TABLE: [Syscall; TABLE_LEN] = [
	sys::read,			// 0
	sys::write,			// 1
	sys::exit,			// 2
	sys::sleep,			// 3
	sys::task_id,		// 4
	sys::placeholder,	// 5
	sys::placeholder,	// 6
	sys::placeholder,	// 7
];

/// Enum representing whether a syscall was successfull or failed.
#[repr(u8)]
pub enum Status {
	/// The call succeeded.
	Ok = 0,
	/// There is no syscall with the given ID (normally used by [´arch´](crate::arch)).
	NoCall,
	/// The resource is temporarily unavailable and the caller should try again
	Retry,
}

impl From<Status> for u8 {
	/// Converts the status to an FFI-safe `u8`
	fn from(status: Status) -> Self {
		// SAFETY: there is no potential for UB here
		unsafe { core::mem::transmute(status) }
	}
}

/// Module containing all the actual syscalls.
mod sys {
	use super::*;

	/// Macro to reduce the typing work for each syscall
	macro_rules! sys {
		{
			$(#[$outer:meta])*
			[$task:pat]
			$fn:ident($a0:pat, $a1:pat, $a2:pat, $a3:pat, $a4:pat, $a5:pat)
			$block:block
		} => {
			$(#[$outer])*
			pub extern "C" fn $fn($a0: usize, $a1: usize, $a2: usize, $a3: usize, $a4: usize, $a5: usize, $task: task::Task) -> Return {
				$block
			}
		};
		{
			$(#[$outer:meta])*
			[$task:pat]
			$fn:ident($a0:pat, $a1:pat, $a2:pat, $a3:pat, $a4:pat)
			$block:block
		} => { sys! { $(#[$outer])* [$task] $fn($a0, $a1, $a2, $a3, $a4, _) { $block } } };
		{
			$(#[$outer:meta])*
			[$task:pat]
			$fn:ident($a0:pat, $a1:pat, $a2:pat, $a3:pat)
			$block:block
		} => { sys! { $(#[$outer])* [$task] $fn($a0, $a1, $a2, $a3, _) { $block } } };
		{
			$(#[$outer:meta])*
			[$task:pat]
			$fn:ident($a0:pat, $a1:pat, $a2:pat)
			$block:block
		} => { sys! { $(#[$outer])* [$task] $fn($a0, $a1, $a2, _) { $block } } };
		{
			$(#[$outer:meta])*
			[$task:pat]
			$fn:ident($a0:pat, $a1:pat)
			$block:block
		} => { sys! { $(#[$outer])* [$task] $fn($a0, $a1, _) { $block } } };
		{
			$(#[$outer:meta])*
			[$task:pat]
			$fn:ident($a0:pat)
			$block:block
		} => { sys! { $(#[$outer])* [$task] $fn($a0, _) { $block } } };
		{
			$(#[$outer:meta])*
			[$task:pat]
			$fn:ident()
			$block:block
		} => { sys! { $(#[$outer])* [$task] $fn(_) { $block } } };
	}

	sys! {
		/// Reads something.
		[_] read() {
			Return(Status::NoCall, 0)
		}
	}

	sys! {
		/// Writes something.
		[_] write(a0, a1, a2) {
			crate::io::uart::default(|uart| {
				use crate::io::Device;
				let s = unsafe { core::slice::from_raw_parts(a1 as *const u8, a2) };
				uart.write(s);
				Return(Status::Ok, a2)
			}).unwrap_or(Return(Status::Retry, 0))
		}
	}

	sys! {
		/// Destroys the current task.
		[_] exit(a0) {
			Return(Status::NoCall, 0)
		}
	}

	sys! {
		/// Sleeps for the given amount of seconds given in `a0` and the amount of nanoseconds in
		/// `a1`.
		// TODO actually sleep instead of just "yield"
		[task] sleep(a0, a1) {
			todo!()
				// TODO undefined reference
			//unsafe { arch::trap_next_task(task) };
		}
	}

	sys! {
		/// Returns the ID of the current task.
		[task] task_id() {
			Return(Status::Ok, task.id() as usize)
		}
	}

	sys! {
		/// Placeholder so that I don't need to update TABLE_LEN constantly.
		[_] placeholder() {
			Return(Status::NoCall, 0)
		}
	}
}
