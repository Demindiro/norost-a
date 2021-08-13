//! Machine-mode interrupt handler.
//!
//! This module contains generic code. Arch-specific code is located in [`arch`](crate::arch)

use crate::arch::vms::{self, VirtualMemorySystem, RWX};
use crate::arch::{self, Map, MapRange, Page, PageData};
use crate::memory::ppn::*;
use crate::task;
use core::convert::TryFrom;
use core::ptr::NonNull;

/// The type of a syscall, specifically the amount and type of arguments it takes.
pub type Syscall = extern "C" fn(
	a0: usize,
	a1: usize,
	a2: usize,
	a3: usize,
	a4: usize,
	a5: usize,
	task: task::Task,
) -> Return;

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
	sys::io_wait,                // 0
	sys::io_set_queues,          // 1
	sys::io_set_notify_handler,  // 2
	sys::mem_alloc,              // 3
	sys::mem_dealloc,            // 4
	sys::mem_get_flags,          // 5
	sys::mem_set_flags,          // 6
	sys::mem_physical_addresses, // 7
	sys::sys_set_interrupt_controller,   // 8
	sys::io_notify_return,       // 9
	sys::sys_reserve_interrupt,  // 10
	sys::task_spawn,             // 11
	sys::dev_dma_alloc,          // 12
	sys::sys_platform_info,      // 13
	sys::sys_direct_alloc,       // 14
	sys::sys_log,                // 15
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

/// Mapping used to spawn new tasks.
#[repr(C)]
pub struct Mapping {
	task_address: *mut PageData,
	typ: u8,
	flags: u8,
	self_address: *mut PageData,
}

/// Module containing all the actual syscalls.
mod sys {
	use super::*;

	const PROTECT_R: usize = 0x1;
	const PROTECT_W: usize = 0x2;
	const PROTECT_X: usize = 0x4;
	#[allow(dead_code)]
	const MEGAPAGE: usize = 0x10;
	#[allow(dead_code)]
	const GIGAPAGE: usize = 0x20;
	#[allow(dead_code)]
	const TERAPAGE: usize = 0x30;

	#[derive(Debug)]
	struct InvalidPageFlags;

	fn decode_rwx_flags(flags: usize) -> Result<RWX, InvalidPageFlags> {
		Ok(match flags & 7 {
			f if f == PROTECT_R => RWX::R,
			f if f == PROTECT_X => RWX::X,
			f if f == PROTECT_R | PROTECT_W => RWX::RW,
			f if f == PROTECT_R | PROTECT_X => RWX::RX,
			f if f == PROTECT_R | PROTECT_W | PROTECT_X => RWX::RWX,
			_ => return Err(InvalidPageFlags),
		})
	}

	macro_rules! logcall {
		($($args:expr),+ $(,)?) => {
			#[cfg(feature = "log-syscalls")]
			log!($($args),+);
			#[cfg(not(feature = "log-syscalls"))]
			let _ = ($($args),+);
		};
	}

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
			logcall!("io_wait 0b{:b}, {}", flags, time);
			// FIXME actually wait for I/O
			// FIXME lol, lmao

			// FIXME this is a quick test to see if we can wake up from idling.
			/*
			if let Some(h) = task.set_notification_handler(None) {
				task.set_notification_handler(Some(h));
			} else {
				task::Executor::idle();
			}
			*/

			crate::task::Executor::default().next()
		}
	}

	sys! {
		/// Resize the task's IPC buffers to be able to hold the given amount of entries.
		[task] io_set_queues(packet_table, mask_bits, free_pages, free_pages_size) {
			logcall!(
				"io_set_queues 0x{:x}, {}, 0x{:x}, {}",
				packet_table,
				mask_bits,
				free_pages,
				free_pages_size,
			);
			let a = match NonNull::new(packet_table as *mut _) {
				Some(pt) => {
					let mb = mask_bits as u8;
					let fp = NonNull::new(free_pages as *mut _);
					let fs = free_pages_size;
					let r = fp.map(|fp| unsafe { crate::task::ipc::IPC::new(pt, mb, fp, fs).unwrap() });
					if r.is_none() {
						return Return(Status::NullArgument, 0);
					}
					r
				}
				None => None,
			};
			task.set_queues(a);
			Return(Status::Ok, 0)
		}
	}

	sys! {
		/// Set a handler for receiving notifications.
		[task] io_set_notify_handler(function) {
			logcall!("io_set_notify_handler 0x{:x}", function);
			let handler = NonNull::new(function as *mut _)
				.map(task::notification::Handler::new)
				.map(Result::unwrap);
			let prev = task.set_notification_handler(handler);
			Return(Status::Ok, prev.map(|p| p.as_ptr() as usize).unwrap_or(0))
		}
	}

	sys! {
		/// Return from a notification handler.
		[task] io_notify_return() {
			logcall!("io_notify_return");
			// TODO we should call this directly
			//
			// The main reason we don't yet is that logcall won't work.
			extern "C" {
				fn syscall_io_notify_return(_: crate::task::Task) -> !;
			}
			unsafe {
				syscall_io_notify_return(task.clone());
			}
		}
	}

	sys! {
		/// Allocates a range of private or shared pages for the current task.
		[_] mem_alloc(address, count, flags) {
			logcall!("mem_alloc 0x{:x}, {}, 0b{:b}", address, count, flags);
			match arch::Page::try_from(address as *mut _) {
				Ok(address) => match decode_rwx_flags(flags) {
					Ok(rwx) => {
						task::Task::allocate_memory(address, count, rwx).unwrap();
						Return(Status::Ok, address.as_ptr() as usize)
					}
					Err(InvalidPageFlags) => Return(Status::MemoryInvalidProtectionFlags, 0),
				}
				Err(arch::page::FromPointerError::Null) => Return(Status::NullArgument, 0),
				Err(arch::page::FromPointerError::BadAlignment) => Return(Status::BadAlignment, 0),
			}
		}
	}

	sys! {
		/// Frees a range of pages of the current task.
		[_] mem_dealloc(address, count) {
			logcall!("mem_dealloc 0x{:x}, {}", address, count);
			let address = match Page::from_usize(address) {
				Ok(a) => a,
				Err(arch::page::FromPointerError::Null) => return Return(Status::NullArgument, 0),
				Err(arch::page::FromPointerError::BadAlignment) => return Return(Status::BadAlignment, 0),
			};
			task::Task::deallocate_memory(address, count).unwrap();
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
			logcall!("mem_physical_addresses 0x{:x}, 0x{:x}, {}", address, store, count);
			if address & arch::PAGE_MASK != 0 {
				return Return(Status::BadAlignment, 0);
			}
			let store = unsafe { core::slice::from_raw_parts_mut(store as *mut _, count) };
			let address = arch::Page::try_from(address as *mut _).unwrap();
			arch::set_supervisor_userpage_access(true);
			let ret = arch::VMS::physical_addresses(address, store);
			arch::set_supervisor_userpage_access(false);
			Return(if ret.is_ok() { Status::Ok } else { Status::MemoryNotAllocated }, 0)
		}
	}

	sys! {
		[_] task_spawn(mappings, mappings_count, program_counter, stack_pointer) {
			logcall!("task_spawn 0x{:x}, {}, 0x{:x}, 0x{:x}", mappings, mappings_count, program_counter, stack_pointer);
			let mappings = unsafe { core::slice::from_raw_parts(mappings as *const Mapping, mappings_count) };
			use crate::task::*;
			let vms = arch::VMS::new().unwrap();
			arch::set_supervisor_userpage_access(true);
			for map in mappings {
				match map.typ {
					// Share mapping from current process.
					0 => {
						let rwx = decode_rwx_flags(map.flags.into()).unwrap();
						logcall!("  share_map  {:p} -> {:p} ({:?})", map.self_address, map.task_address, rwx);
						vms.share(
							arch::Page::try_from(map.task_address).unwrap(),
							arch::Page::try_from(map.self_address).unwrap(),
							rwx,
							vms::Accessibility::UserLocal,
						).unwrap()
					}
					// Invalid type
					_ => todo!(),
				}
			}
			arch::set_supervisor_userpage_access(false);
			let task = Task::new(vms).unwrap();
			task.set_pc(program_counter as *const ());
			let group = Group::get(0).unwrap();
			let id = group.insert(task).unwrap();
			Return(Status::Ok, id)
		}
	}

	sys! {
		[_] dev_dma_alloc(address, size, _flags) {
			logcall!("dev_dma_alloc 0x{:x}, {}, 0b{:b}", address, size, _flags);
			assert_ne!(size, 0, "TODO just return an error doof");
			// FIXME this should be in the PMM
			let mut ppns = [None, None, None, None, None, None, None, None];
			let count = (size + arch::Page::SIZE - 1) / arch::Page::SIZE;
			use crate::memory;
			ppns[0] = Some(memory::allocate().unwrap());
			for i in 1..count {
				ppns[i] = Some(memory::allocate().unwrap());
			}
			if let Some(addr) = NonNull::new(address as *mut _) {
				let mut addr = arch::Page::new(addr).ok();
				for i in 0..count {
					if let Some(a) = addr {
						let p = core::mem::replace(&mut ppns[i], None).unwrap();
						let p = Map::Private(p);
						arch::VMS::add(a, p, vms::RWX::RW, vms::Accessibility::UserLocal)
							.unwrap();
						addr = a.next();
					} else {
						todo!();
					}
				}
				Return(Status::Ok, address)
			} else {
				todo!()
			}
		}
	}

	sys! {
		[_] sys_platform_info(address, _max_count) {
			logcall!("sys_platform_info 0x{:x}, {}", address, _max_count);
			use crate::{PLATFORM_INFO_SIZE, PLATFORM_INFO_PHYS_PTR};
			if let Some(a) = NonNull::new(address as *mut _) {
				if let Ok(a) = arch::Page::new(a) {
					let p = PPNDirect::from_usize(*PLATFORM_INFO_PHYS_PTR).unwrap();
					if let Ok(p) = PPNDirectRange::new(p.into(), *PLATFORM_INFO_SIZE) {
						let p = MapRange::Direct(p);
						arch::VMS::add_range(a, p, vms::RWX::R, vms::Accessibility::UserLocal).unwrap();
						Return(Status::Ok, *PLATFORM_INFO_SIZE)
					} else {
						todo!()
					}
				} else {
					Return(Status::BadAlignment, 0)
				}
			} else {
				Return(Status::NullArgument, 0)
			}
		}
	}

	sys! {
		[_] sys_direct_alloc(address, ppn, count, _flags) {
			logcall!("sys_direct_alloc 0x{:x}, 0x{:x}, {}, 0b{:b}", address, ppn << arch::PAGE_BITS, count, _flags);
			if let Some(addr) = NonNull::new(address as *mut _) {
				if let Ok(addr) = arch::Page::new(addr) {
					if let Ok(ppn) = PPNBox::try_from(ppn) {
						if let Ok(ppn) = PPNDirectRange::new(ppn, count) {
							let map = MapRange::Direct(ppn);
							match arch::VMS::add_range(addr, map, RWX::RW, vms::Accessibility::UserLocal) {
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
			} else {
				Return(Status::NullArgument, 0)
			}
		}
	}

	sys! {
		/// Reserve an interrupt source. This will cause any interrupts from the given sources to
		/// cause a notification to be sent to the calling task.
		[_] sys_reserve_interrupt(interrupt) {
			let address = task::Executor::current_address();
			arch::interrupts::reserve(interrupt as u16, address).unwrap();
			Return(Status::Ok, 0)
		}
	}

	sys! {
		/// Put a message in the kernel's stdout. Intended for low-level debugging.
		[_] sys_log(address, length) {
			logcall!("sys_log 0x{:x}, {}", address, length);
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
			let _ = write!(Log, "{:?}", BrokenStr(unsafe { slice::from_raw_parts(address as *const _, length) }));
			arch::set_supervisor_userpage_access(false);

			Return(Status::Ok, 0)
		}
	}

	sys! {
		/// Set the MMIO region where the interrupt controller is located.
		///
		/// This may only be called once.
		[_] sys_set_interrupt_controller(ppn, page_count, max_devices) {
			logcall!("sys_set_interrupt_controller 0x{:x}, {}", (ppn as u128) << 12, page_count);
			log!("sys_set_interrupt_controller 0x{:x}, {}", (ppn as u128) << 12, page_count);
			use crate::memory::reserved::PLIC;
			assert!(page_count <= PLIC.page_count());
			let ppn = PPNBox::try_from(ppn).unwrap();
			let ppn_range = PPNDirectRange::new(ppn, page_count).unwrap();
			let max_devices = max_devices as u16;
			unsafe { arch::interrupts::set_controller(MapRange::Direct(ppn_range), max_devices) };
			Return(Status::Ok, 0)
		}
	}

	sys! {
		/// Placeholder so that I don't need to update TABLE_LEN constantly.
		[_] placeholder() {
			logcall!("placeholder");
			Return(Status::InvalidCall, 0)
		}
	}
}
