//! Machine-mode interrupt handler.
//!
//! This module contains generic code. Arch-specific code is located in [`arch`](crate::arch)

use crate::arch;
use crate::task;
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
	sys::dev_reserve,			// 10
	sys::placeholder,			// 11
	sys::dev_dma_alloc,			// 12
	sys::placeholder,			// 13
	sys::placeholder,			// 14
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
		[task] dev_reserve(id, a_reg, a_ranges, a_ranges_count) {
			arch::set_supervisor_userpage_access(true);
			let mut a_ranges = unsafe {
				core::slice::from_raw_parts(a_ranges as *const *mut arch::Page, a_ranges_count)
			};
			log!("dev_reserve {}, 0x{}, {:?}", id, a_reg, a_ranges);
			use crate::driver::DeviceTree;
			use crate::memory::reserved::DEVICE_TREE;

			let dt = unsafe { DeviceTree::parse_dtb(DEVICE_TREE.start.as_ptr()).unwrap() };
			let mut int = dt.interpreter();

			let mut address_cells = 0;
			let mut size_cells = 0;

			while let Some(mut node) = int.next_node() {
				while let Some(p) = node.next_property() {
					match p.name {
						"#address-cells" => {
							let num = p.value.try_into().expect("Malformed #address-cells");
							address_cells = u32::from_be_bytes(num);
						}
						"#size-cells" => {
							let num = p.value.try_into().expect("Malformed #size-cells");
							size_cells = u32::from_be_bytes(num);
						}
						_ => (),
					}
				}
				use crate::memory::PPNRange;
				while let Some(mut node) = node.next_child_node() {
					if node.name == "soc" {
						while let Some(p) = node.next_property() {}
						while let Some(mut node) = node.next_child_node() {
							let mut compatible = false;
							let mut ranges = None;
							let mut reg = None;
							let mut child_address_cells = address_cells;
							let mut child_size_cells = size_cells;
							while let Some(p) = node.next_property() {
								match p.name {
									"compatible" => compatible = p.value == b"pci-host-ecam-generic\0",
									"ranges" => ranges = Some(p.value),
									"reg" => reg = Some(p.value),
									"#address-cells" => child_address_cells = u32::from_be_bytes(p.value.try_into().unwrap()),
									"#size-cells" => child_size_cells = u32::from_be_bytes(p.value.try_into().unwrap()),
									_ => (),
								}
							}
							if !compatible {
								continue;
							}
							dbg!(node.name);
							log!("ranges {:?}", ranges);
							log!("reg {:?}", reg);

							// Map regions into address space.
							let mut addr = NonNull::new(a_reg as *mut arch::Page).expect("Address is 0");

							// Map reg first
							let reg = reg.expect("No reg property");
							let (start, reg): (usize, _) = match address_cells {
								1 => (u32::from_be_bytes(reg[..4].try_into().unwrap()).try_into().unwrap(), &reg[4..]),
								2 => (u64::from_be_bytes(reg[..8].try_into().unwrap()).try_into().unwrap(), &reg[8..]),
								_ => panic!("Address cell size too large"),
							};
							let size: usize = match size_cells {
								1 => u32::from_be_bytes(reg.try_into().unwrap()).try_into().unwrap(),
								2 => u64::from_be_bytes(reg.try_into().unwrap()).try_into().unwrap(),
								_ => panic!("Size cell size too large"),
							};
							let ppn = unsafe { PPNRange::from_ptr(start, (size / arch::PAGE_SIZE).try_into().unwrap()) };
							arch::VirtualMemorySystem::add_range(addr, ppn, arch::RWX::RW, true, false);

							// Map ranges
							let mut ranges = ranges.expect("No ranges property");
							while ranges.len() > 0 {
								let mut addr = NonNull::new(a_ranges[0] as *mut arch::Page).unwrap();
								a_ranges = &a_ranges[1..];
								let r = &ranges[child_address_cells as usize * 4..];
								let (start, r): (usize, _) = match address_cells {
									1 => (u32::from_be_bytes(r[..4].try_into().unwrap()).try_into().unwrap(), &r[4..]),
									2 => (u64::from_be_bytes(r[..8].try_into().unwrap()).try_into().unwrap(), &r[8..]),
									_ => panic!("Address cell size too large"),
								};
								let (size, r): (usize, _) = match size_cells {
									1 => (u32::from_be_bytes(r[..4].try_into().unwrap()).try_into().unwrap(), &r[4..]),
									2 => (u64::from_be_bytes(r[..8].try_into().unwrap()).try_into().unwrap(), &r[8..]),
									_ => panic!("Size cell size too large"),
								};
								ranges = r;
								let ppn = unsafe { PPNRange::from_ptr(start, (size / arch::PAGE_SIZE).try_into().unwrap()) };
								arch::VirtualMemorySystem::add_range(addr, ppn, arch::RWX::RW, true, false);
							}
						}
					}
				}
			}

			arch::set_supervisor_userpage_access(false);
			Return(Status::Ok, 0)
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
				dbg!(i, &p);
				let a = NonNull::new(a.wrapping_add(i)).unwrap();
				arch::VirtualMemorySystem::add(a, p, arch::RWX::RW, true, false);
			}
			Return(Status::Ok, a as usize)
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
