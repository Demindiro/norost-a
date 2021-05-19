//! Machine-mode interrupt handler.
//!
//! This module contains generic code. Arch-specific code is located in [`arch`](crate::arch)

use crate::arch;
use core::convert::TryFrom;

/// The type of a syscall, specifically the amount and type of arguments it takes.
pub type Syscall = extern "C" fn(a0: usize, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize) -> Return;

/// The FFI-safe return value of syscalls
///
/// - The first field is the status of the call (i.e. did it succeed or was there an error).
/// - The second field is optional extra data that may be attached, such as a file descriptor.
#[repr(C)]
pub struct Return(Status, usize);

/// The length of the table as a separate constant because Rust is a little dum dum.
pub const TABLE_LEN: usize = 2;

/// Table with all syscalls.
#[allow(non_upper_case_globals)]
#[no_mangle]
//pub static TABLE: [Syscall; TABLE_LEN] = [
pub static syscall_table: [Syscall; TABLE_LEN] = [
	sys::read,
	sys::write,
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
			$fn:ident($a0:ident, $a1:ident, $a2:ident, $a3:ident, $a4:ident, $a5:ident)
			$block:block
		} => {
			$(#[$outer])*
			pub extern "C" fn $fn($a0: usize, $a1: usize, $a2: usize, $a3: usize, $a4: usize, $a5: usize) -> Return {
				$block
			}
		};
	}

	sys! {
		/// Reads something.
		read(a0, a1, a2, a3, a4, a5) {
			Return(Status::NoCall, 0)
		}
	}

	sys! {
		/// Writes something.
		write(a0, a1, a2, a3, a4, a5) {
			crate::io::uart::default(|uart| {
				use crate::io::Device;
				let s = unsafe { core::slice::from_raw_parts(a1 as *const u8, a2) };
				uart.write(s);
				Return(Status::Ok, a2)
			}).unwrap_or(Return(Status::Retry, 0))
		}
	}
}
