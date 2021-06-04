//! Machine-mode interrupt handler.
//!
//! This module contains generic code. Arch-specific code is located in [`arch`](crate::arch)

use crate::{arch, task};
use core::convert::TryFrom;
use core::num::NonZeroU8;
use core::ptr::NonNull;

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
	sys::io_wait,				// 0
	sys::io_resize_requester,	// 1
	sys::io_resize_responder,	// 2
	sys::mem_alloc,				// 3
	sys::mem_dealloc,			// 4
	sys::mem_get_flags,			// 5
	sys::mem_set_flags,			// 6
	sys::placeholder,			// 7
];

/// Enum representing whether a syscall was successfull or failed.
#[repr(u8)]
pub enum Status {
	/// The call succeeded.
	Ok = 0,
	/// There is no syscall with the given ID (normally used by [´arch´](crate::arch)).
	InvalidCall = 1,
	/// One of the arguments is `None` when it shouldn't be.
	NullArgument = 2,
	/// The address range overlaps with an existing range.
	MemoryOverlap = 3,
	/// There is no more memory available.
	MemoryUnavailable = 4,
	/// The flags of one or more memory pages are locked.
	MemoryLocked = 5,
	/// The memory at the address is not allocated (i.e. it doesn't exist).
	MemoryNotAllocated = 6,
	/// THe combination of protection flags is invalid.
	MemoryInvalidProtectionFlags = 7,
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
		/// Waits for one or all I/O events to complete
		[task] io_wait(flags, time) {
			crate::log::debug!("io_wait 0b{:b}, {}", flags, time);
			// FIXME actually wait for I/O
			unsafe { crate::arch::trap_next_task(task); }
		}
	}

	sys! {
		/// Resize the task's requester buffers to be able to hold the given amount of entries.
		[task] io_resize_requester(request_queue, request_size, completion_queue, completion_size) {
			crate::log::debug!(
				"io_resize_requester 0x{:x}, {}, 0x{:x}, {}", 
				request_queue,
				request_size,
				completion_queue,
				completion_size,
			);
			let a = if request_queue == 0 {
				None
			} else {
				let rq = NonNull::new(request_queue as *mut _);
				let rs = request_size as u8;
				let cq = NonNull::new(completion_queue as *mut _);
				let cs = completion_size as u8;
				let r = rq.and_then(|rq| cq.map(|cq| ((rq, rs), (cq, cs))));
				if r.is_none() {
					return Return(Status::NullArgument, 0);
				}
				r
			};
			task.set_client_buffers(a);
			Return(Status::Ok, 0)
		}
	}

	sys! {
		/// Resize the task's requester buffers to be able to hold the given amount of entries.
		[_] io_resize_responder(a0) {
			todo!();
		}
	}

	sys! {
		/// Allocates a range of private or shared pages for the current task.
		[task] mem_alloc(address, count, flags) {
			const PROTECT_R: usize = 0x1;
			const PROTECT_W: usize = 0x2;
			const PROTECT_X: usize = 0x4;
			const SHAREABLE: usize = 0x8;
			const MEGAPAGE: usize = 0x10;
			const GIGAPAGE: usize = 0x20;
			const TERAPAGE: usize = 0x30;
			crate::log::debug!("mem_alloc 0x{:x}, {}, 0b{:b}", address, count, flags);
			use crate::arch::RWX;
			let address = match NonNull::new(address as *mut _) {
				Some(a) => a,
				None => return Return(Status::NullArgument, 0),
			};
			let rwx = match flags & 7 {
				PROTECT_R => RWX::R,
				PROTECT_X => RWX::X,
				f if f == PROTECT_R | PROTECT_W => RWX::RW,
				f if f == PROTECT_R | PROTECT_X => RWX::RX,
				f if f == PROTECT_R | PROTECT_W | PROTECT_X => RWX::RWX,
				_ => return Return(Status::MemoryInvalidProtectionFlags, 0),
			};
			if flags & SHAREABLE > 0 {
				task.allocate_shared_memory(address, count, rwx).unwrap();
			} else {
				task.allocate_memory(address, count, rwx).unwrap();
			}
			Return(Status::Ok, address.as_ptr() as usize)
		}
	}

	sys! {
		/// Frees a range of pages of the current task.
		[task] mem_dealloc(address, count) {
			let address = match NonNull::new(address as *mut _) {
				Some(a) => a,
				None => return Return(Status::NullArgument, 0),
			};
			task.deallocate_memory(address, count).unwrap();
			Return(Status::Ok, 0)
		}
	}

	sys! {
		[task] mem_get_flags() {
			todo!()
		}
	}

	sys! {
		[task] mem_set_flags() {
			todo!()
		}
	}

	sys! {
		/// Placeholder so that I don't need to update TABLE_LEN constantly.
		[_] placeholder() {
			Return(Status::InvalidCall, 0)
		}
	}
}
