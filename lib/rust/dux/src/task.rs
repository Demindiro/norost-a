//! # Helper functions to spawn & manage tasks

use crate::mem;
use crate::{Page, RWX};
use core::fmt;
use core::slice;

/// A task address
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Address(usize);

impl Address {
	/// The (pseudo) address of the kernel. This is used for notifications sent by the kernel.
	pub const KERNEL: Self = Self(usize::MAX);

	/// The default invalid address. This is the same as the kernel address.
	///
	/// It is used by some functions to indicate the address of the calling task should be used,
	/// such as `registry::get`.
	pub const INVALID: Self = Self(usize::MAX);
}

impl fmt::Debug for Address {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let s = core::mem::size_of::<Self>() * 4;
		f.debug_struct("Address")
			.field("group", &(self.0 >> s))
			.field("task", &((self.0 << s) >> s))
			.finish()
	}
}

impl fmt::Display for Address {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let s = core::mem::size_of::<Self>() * 4;
		(self.0 >> s).fmt(f)?;
		"|".fmt(f)?;
		((self.0 << s) >> s).fmt(f)
	}
}

#[derive(Debug)]
pub enum SpawnElfError {
	ReserveError(mem::ReserveError),
	BadRWXFlags,
}

/// Create a new task from an ELF file.
pub fn spawn_elf(data: &[kernel::Page]) -> Result<Address, SpawnElfError> {
	use xmas_elf::ElfFile;

	// SAFETY: the data is guaranteed to be properly aligned and have the proper size
	let data =
		unsafe { core::slice::from_raw_parts(data.as_ptr().cast::<u8>(), data.len() * Page::SIZE) };

	// This struct ensures no memory is leaked.
	struct DropRanges([Option<(Page, usize)>; 8], usize);

	impl Drop for DropRanges {
		fn drop(&mut self) {
			for (addr, count) in self.0[..self.1].iter().copied().map(Option::unwrap) {
				// SAFETY: Nothing can (should?) be using these ranges anymore.
				unsafe { mem::deallocate_range(addr, count) };
			}
		}
	}

	// SAFETY: all zeroes TaskSpawnMapping is valid.
	let mut mappings =
		unsafe { core::mem::MaybeUninit::<[kernel::TaskSpawnMapping; 64]>::zeroed().assume_init() };
	let mut i = 0;
	let mut pc = 0;

	let mut reserved_ranges = DropRanges([None; 8], 0);

	let elf = ElfFile::new(data).unwrap();
	for ph in elf
		.program_iter()
		.filter(|ph| ph.get_type() == Ok(xmas_elf::program::Type::Load))
	{
		let mut offset = ph.offset() as usize & !Page::OFFSET_MASK;
		let mut page_offset = ph.offset() as usize & Page::OFFSET_MASK;
		let mut virt_a = ph.virtual_addr() as usize & !Page::OFFSET_MASK;

		let file_pages = ((ph.file_size() as usize + page_offset + Page::OFFSET_MASK)
			& !Page::OFFSET_MASK)
			/ Page::SIZE;
		let mem_pages = ((ph.mem_size() as usize + page_offset + Page::OFFSET_MASK)
			& !Page::OFFSET_MASK)
			/ Page::SIZE;
		let flags = ph.flags();
		let flags = match (flags.is_read(), flags.is_write(), flags.is_execute()) {
			(true, false, false) => RWX::R,
			(false, true, false) => RWX::W,
			(false, false, true) => RWX::X,
			(true, true, false) => RWX::RW,
			(true, false, true) => RWX::RX,
			(true, true, true) => RWX::RWX,
			_ => Err(SpawnElfError::BadRWXFlags)?,
		};

		if ph.flags().is_write() {
			// We must copy the pages as they may be written to.
			let addr =
				mem::allocate_range(None, mem_pages, flags).map_err(SpawnElfError::ReserveError)?;

			reserved_ranges.0[reserved_ranges.1] = Some((addr, mem_pages));
			reserved_ranges.1 += 1;

			let addr = addr.as_ptr().cast::<u8>();

			let data = match ph {
				xmas_elf::program::ProgramHeader::Ph64(ph) => ph.raw_data(&elf),
				_ => unreachable!(),
			};
			for k in 0..ph.file_size() {
				unsafe { *addr.add(k as usize) = data[k as usize] };
			}
			for k in 0..mem_pages {
				let self_address = addr.wrapping_add((k * Page::SIZE) as usize) as *mut _;
				mappings[i] = kernel::TaskSpawnMapping {
					typ: 0,
					flags: flags.into(),
					task_address: virt_a as *mut _,
					self_address,
				};
				i += 1;
				offset += Page::SIZE;
				virt_a += Page::SIZE;
			}
		} else {
			// It is safe to share the pages
			for _ in 0..file_pages {
				let self_address = data.as_ptr().wrapping_add(offset as usize) as *mut _;
				mappings[i] = kernel::TaskSpawnMapping {
					typ: 0,
					flags: flags.into(),
					task_address: virt_a as *mut _,
					self_address,
				};
				i += 1;
				offset += Page::SIZE;
				virt_a += Page::SIZE;
			}
		}
	}

	// Allocate a stack
	{
		let stack_pages = 16;
		let addr =
			mem::allocate_range(None, stack_pages, RWX::RW).map_err(SpawnElfError::ReserveError)?;

		reserved_ranges.0[reserved_ranges.1] = Some((addr, stack_pages));
		reserved_ranges.1 += 1;

		let mut virt_a = 0x7fff_0000;
		let mut offset = 0x0;

		for i in i..i + 16 {
			let self_address = addr.as_ptr().wrapping_add(offset as usize) as *mut _;
			mappings[i] = kernel::TaskSpawnMapping {
				typ: 0,
				flags: RWX::RW.into(),
				task_address: virt_a as *mut _,
				self_address,
			};
			offset += 1;
			virt_a += Page::SIZE;
		}
		i += 16;
	}

	let pc = elf.header.pt2.entry_point() as usize;

	let ret = unsafe {
		kernel::task_spawn(
			mappings.as_ptr(),
			i,
			pc as *const _,
			0x8000_0000 as *const _,
		)
	};
	match ret.status {
		kernel::Return::OK => Ok(Address(ret.value)),
		r => unreachable!("{}", r),
	}
}

pub mod registry {

	use super::Address;

	#[derive(Debug)]
	pub enum AddError {
		Unavailable,
		NameTooLong,
		Occupied,
	}

	#[derive(Debug)]
	pub enum GetError {
		NotFound,
	}

	/// Try to add a task to the kernel's registry.
	pub fn add(name: &[u8], address: Address) -> Result<(), AddError> {
		let ret = unsafe { kernel::sys_registry_add(name.as_ptr(), name.len(), address.0) };
		match ret.status {
			kernel::Return::OK => Ok(()),
			kernel::Return::MEMORY_UNAVAILABLE => Err(AddError::Unavailable),
			kernel::Return::TOO_LONG => Err(AddError::NameTooLong),
			kernel::Return::OCCUPIED => Err(AddError::Occupied),
			r => unreachable!("{}", r),
		}
	}

	/// Find a task in the kernel's registry.
	pub fn get(name: &[u8]) -> Result<Address, GetError> {
		// Check if we can find the added entry.
		let ret = unsafe { kernel::sys_registry_get(name.as_ptr(), name.len()) };
		match ret.status {
			kernel::Return::OK => Ok(Address(ret.value)),
			kernel::Return::NOT_FOUND => Err(GetError::NotFound),
			r => unreachable!("{}", r),
		}
	}
}
