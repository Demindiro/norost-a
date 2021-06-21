//! Machine-mode interrupt handler.
//!
//! This module contains generic code. Arch-specific code is located in [`arch`](crate::arch)

use crate::arch::{self, Map, MapRange, VirtualMemorySystem, RWX};
use crate::task;
use crate::memory::ppn::*;
use core::convert::{TryFrom, TryInto};
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
pub const TABLE_LEN: usize = 16;

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
	sys::mem_physical_addresses,// 7
	sys::placeholder,			// 8
	sys::placeholder,			// 9
	sys::placeholder,			// 10
	sys::placeholder,			// 11
	sys::dev_dma_alloc,			// 12
	sys::sys_platform_info,		// 13
	sys::sys_direct_alloc,		// 14
	sys::sys_log,				// 15
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
	/// The address isn't properly aligned.
	BadAlignment = 8,
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
			log!("io_wait 0b{:b}, {}", flags, time);
			// FIXME actually wait for I/O
			unsafe { crate::arch::trap_next_task(task); }
		}
	}

	sys! {
		/// Resize the task's requester buffers to be able to hold the given amount of entries.
		[task] io_resize_requester(request_queue, request_size, completion_queue, completion_size) {
			log!(
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
		[_] io_resize_responder() {
			todo!();
		}
	}

	sys! {
		/// Allocates a range of private or shared pages for the current task.
		[task] mem_alloc(address, count, flags) {
			const PROTECT_R: usize = 0x1;
			const PROTECT_W: usize = 0x2;
			const PROTECT_X: usize = 0x4;
			#[allow(dead_code)]
			const MEGAPAGE: usize = 0x10;
			#[allow(dead_code)]
			const GIGAPAGE: usize = 0x20;
			#[allow(dead_code)]
			const TERAPAGE: usize = 0x30;
			log!("mem_alloc 0x{:x}, {}, 0b{:b}", address, count, flags);
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
			task.allocate_memory(address, count, rwx).unwrap();
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
		[_] mem_get_flags() {
			todo!()
		}
	}

	sys! {
		[_] mem_set_flags() {
			todo!()
		}
	}

	sys! {
		[_] mem_physical_addresses(address, store, count) {
			log!("mem_physical_addresses 0x{:x}, 0x{:x}, {}", address, store, count);
			if address & arch::PAGE_MASK != 0 {
				return Return(Status::BadAlignment, 0);
			}
			let store = unsafe { core::slice::from_raw_parts_mut(store as *mut _, count) };
			let address = NonNull::new(address as *mut _).unwrap();
			arch::set_supervisor_userpage_access(true);
			let ret = arch::VirtualMemorySystem::physical_addresses(address, store);
			arch::set_supervisor_userpage_access(false);
			Return(if ret.is_ok() { Status::Ok } else { Status::MemoryNotAllocated }, 0)
		}
	}

	sys! {
		[_] dev_dma_alloc(address, size, _flags) {
			log!("dev_dma_alloc 0x{:x}, {}, 0b{:b}", address, size, _flags);
			assert_ne!(size, 0, "TODO just return an error doof");
			// FIXME this should be in the PMM
			let mut ppns = [None, None, None, None, None, None, None, None];
			let count = (size + arch::PAGE_SIZE - 1) / arch::PAGE_SIZE;
			use crate::memory;
			ppns[0] = Some(memory::allocate().unwrap());
			dbg!(size, count);
			for i in 1..count {
				ppns[i] = Some(memory::allocate().unwrap());
			}
			let a = address as *mut arch::Page;
			for i in 0..count {
				let p = core::mem::replace(&mut ppns[i], None).unwrap();
				let p = Map::Private(p);
				let a = NonNull::new(a.wrapping_add(i)).unwrap();
				arch::VirtualMemorySystem::add(a, p, arch::RWX::RW, true, false);
			}
			Return(Status::Ok, a as usize)
		}
	}

	sys! {
		[_] sys_platform_info(address, _max_count) {
			log!("sys_platform_info 0x{:x}, {}", address, _max_count);
			use crate::{PLATFORM_INFO_SIZE, PLATFORM_INFO_PHYS_PTR};
			if let Some(a) = NonNull::new(address as *mut arch::Page) {
				let p = PPNDirect::from_usize(*PLATFORM_INFO_PHYS_PTR).unwrap();
				if let Ok(p) = PPNDirectRange::new(p.into(), *PLATFORM_INFO_SIZE) {
					let p = MapRange::Direct(p);
					arch::VirtualMemorySystem::add_range(a, p, arch::RWX::R, true, false).unwrap();
					Return(Status::Ok, *PLATFORM_INFO_SIZE)
				} else {
					todo!()
				}
			} else {
				Return(Status::NullArgument, 0)
			}
		}
	}

	sys! {
		[_] sys_direct_alloc(address, ppn, count, _flags) {
			log!("sys_direct_alloc 0x{:x}, 0x{:x}, {}, 0b{:b}", address, ppn << arch::PAGE_BITS, count, _flags);
			if let Some(addr) = NonNull::new(address as *mut _) {
				if let Ok(ppn) = PPNBox::try_from(ppn) {
					if let Ok(ppn) = PPNDirectRange::new(ppn, count) {
						let map = MapRange::Direct(ppn);
						match VirtualMemorySystem::add_range(addr, map, RWX::RW, true, false) {
							Ok(()) => Return(Status::Ok, 0),
							Err(_) => Return(Status::MemoryOverlap, 0),
						}
					} else {
						todo!()
						//Return(Status::, 0)
					}
				} else {
					Return(Status::MemoryUnavailable, 0)
				}
			} else {
				Return(Status::NullArgument, 0)
			}
		}
	}

	sys! {
		/// Put a message in the kernel's stdout. Intended for low-level debugging.
		[_] sys_log(address, length) {
			// Replace any non-valid UTF-8 characters
			struct BrokenStr<'a>(&'a [u8]);

			use core::{fmt, str, slice};
			impl fmt::Debug for BrokenStr<'_> {
				fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
					let mut i = 0;
					while i < self.0.len() {
						match str::from_utf8(&self.0[i..]) {
							Ok(s) => {
								f.write_str(s)?;
								break;
							}
							Err(e) => {
								if let Some(l) = e.error_len() {
									let v = e.valid_up_to();
									f.write_str(str::from_utf8(&self.0[i..e.valid_up_to()]).unwrap())?;
									f.write_str("\u{FFFD}")?;
									i += v + l;
								} else {
									break;
								}
							}
						}
					}
					Ok(())
				}
			}

			// TODO handle pagefaults
			arch::set_supervisor_userpage_access(true);
			use crate::log::Log;
			use core::fmt::Write;
			write!(Log, "{:?}", BrokenStr(unsafe { slice::from_raw_parts(address as *const _, length) }));
			arch::set_supervisor_userpage_access(false);

			Return(Status::Ok, 0)
		}
	}

	sys! {
		/// Placeholder so that I don't need to update TABLE_LEN constantly.
		[_] placeholder() {
			log!("placeholder");
			Return(Status::InvalidCall, 0)
		}
	}
}
