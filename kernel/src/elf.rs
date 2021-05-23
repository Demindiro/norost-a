//! Module to parse & load ELF executables.
//!
//! This module is necessary to execute `/init` on the `initramfs`. As that is its only purpose,
//! it's limited to loading executables of the appropriate format (ELF32 for 32 bit systems, ELF64
//! for 64 bit).
//!
//! ## References
//!
//! [ELF-64 Object File Format][elf64]
//!
//! [elf64]: https://uclibc.org/docs/elf-64-gen.pdf

use crate::alloc::Box;
use crate::log;
use crate::{arch, memory, MEMORY_MANAGER};
use core::alloc::{AllocError, Allocator};
use core::ptr::NonNull;
use core::{mem, ptr};

/// Structure representing an ELF file.
pub struct ELF<A>
where
	A: Allocator,
{
	/// The entry point in the program.
	entry: usize,
	/// The program's segments.
	segments: Box<[Segment], A>,
}

#[cfg_attr(
	target_pointer_width = "32",
	doc = "Structure representing an ELF32 file header"
)]
#[cfg_attr(
	target_pointer_width = "64",
	doc = "Structure representing an ELF64 file header"
)]
#[repr(C)]
struct FileHeader {
	/// The file identifier / "magic".
	identifier: Identifier,
	/// The ELF object type.
	typ: u16,
	/// The targeted architecture.
	machine: u16,
	/// The ELF version.
	version: u32,
	/// The entry point of the program.
	entry: usize,
	program_header_offset: usize,
	section_header_offset: usize,
	/// Architecture-specific flags.
	flags: u32,
	header_size: u16,
	/// The size of each entry in the program header.
	program_header_entry_size: u16,
	/// The amount of entries in the program header.
	program_header_entry_count: u16,
	section_header_entry_size: u16,
	section_header_entry_count: u16,
	section_header_str_rndx: u16,
}
#[cfg(target_pointer_width = "32")]
const _FILE_HEADER_SIZE_CHECK: usize = 0 - (52 - mem::size_of::<FileHeader>());
#[cfg(target_pointer_width = "64")]
const _FILE_HEADER_SIZE_CHECK: usize = 0 - (64 - mem::size_of::<FileHeader>());

/// Structure representing an ELF identifier, located at the start of a header.
#[repr(C)]
struct Identifier {
	/// Magic that must be `b"\x7fELF"`
	magic: [u8; 4],
	class: u8,
	data: u8,
	version: u8,
	_padding: [u8; 9],
}
const _IDENTIFIER_SIZE_CHECK: usize = 0 - (16 - mem::size_of::<Identifier>());

/*
#[cfg_attr(
	target_pointer_width = "32",
	doc = "Structure representing an entry in an ELF32 program header"
)]
#[cfg(target_pointer_width = "32")]
#[repr(C)]
struct ProgramHeader {
	/// The type of the segment.
	typ: u32,
	/// The offset of this segment in the file.
	offset: usize,
	/// The location of this segment in virtual memory.
	virtual_address: usize,
	/// The location of this segment in physical memory.
	physical_address: usize,
	/// The size of the segment.
	size: usize,
	/// Segment flags.
	flags: u32,
	/// The required alignment of this segment, which must be a power of 2.
	alignment: usize,
}
*/
#[cfg_attr(
	target_pointer_width = "64",
	doc = "Structure representing an entry in an an ELF64 program header"
)]
#[cfg(target_pointer_width = "64")]
#[repr(C)]
struct ProgramHeader {
	/// The type of the segment.
	typ: u32,
	/// Segment flags.
	flags: u32,
	/// The offset of this segment in the file.
	offset: usize,
	/// The location of this segment in virtual memory.
	virtual_address: usize,
	/// The location of this segment in physical memory.
	physical_address: usize,
	/// The size of the segment in the file.
	file_size: usize,
	/// The size of the segment in the memory.
	memory_size: usize,
	/// The required alignment of this segment, which must be a power of 2.
	alignment: usize,
}
#[cfg(target_pointer_width = "32")]
const _PROGRAM_HEADER_SIZE_CHECK: usize = 0 - (32 - mem::size_of::<ProgramHeader>());
#[cfg(target_pointer_width = "64")]
const _PROGRAM_HEADER_SIZE_CHECK: usize = 0 - (56 - mem::size_of::<ProgramHeader>());

/// A structure representing a single segment.
struct Segment {
	/// A pointer to the memory page used by this segment
	page: NonNull<arch::Page>,
	/// The virtual address of the start of this segment.
	virtual_address: usize,
	/// The order (i.e. size) of the allocated page.
	order: u8,
	/// The flags of this segment (i.e. whether it's readable, executable, ...).
	flags: u8,
}

/// Error that may be returned when trying to parse an ELF file.
#[derive(Debug)]
pub enum ParseError {
	/// The magic is invalid, i.e. it doesn't start with the string `"\x7fELF"` or it is less than
	/// 16 bytes.
	BadMagic,
	/// The class is unsupported. A 64 bit kernel will only support ELF64 while a 32 bit kernel
	/// will only support ELF32 to keep the kernel small. Generally you shouldn't use a 32 bit init
	/// with a 64 bit kernel anyways (and vice versa won't even run).
	UnsupportedClass,
	/// The data format (i.e. endianness) is unsupported. Again, this is arch-specific.
	UnsupportedData,
	/// The version is unsupported. The only ELF version in existence right now is version 1.
	UnsupportedVersion,
	/// The data isn't properly aligned. Must be on a 4 byte boundary for ELF32 and 8 for ELF64.
	BadAlignment,
	/// The header is too small.
	HeaderTooSmall,
	/// The ELF's object type isn't supported (i.e. it isn't an executable).
	UnsupportedType,
	/// The architecture is not supported.
	UnsupportedMachine,
	/// Some of the flags aren't supported by this architecture.
	UnsupportedFlags,
	/// The program headers don't have the right size.
	BadProgramHeaderSize,
	/// The program headers occupy more space than there is in the file.
	ProgramHeadersLargerThanFile,
	/// An error occured while trying to allocate heap memory.
	AllocError(AllocError),
	/// An error occured while trying to allocate memory pages.
	AllocateError(memory::AllocateError),
}

const TYPE_EXEC: u16 = 2;

impl<A> ELF<A>
where
	A: Allocator,
{
	/// Attempts to parse the given ELF data.
	pub fn parse(data: &[u8], allocator: A) -> Result<Self, ParseError> {
		// Parse the file header

		if data.len() < 16 {
			return Err(ParseError::BadMagic);
		}

		// SAFETY: the data is at least 16 bytes long
		let identifier = unsafe { &*(data as *const [u8] as *const Identifier) };

		if &identifier.magic != b"\x7fELF" {
			return Err(ParseError::BadMagic);
		}

		if data.as_ptr().align_offset(mem::size_of::<usize>()) != 0 {
			return Err(ParseError::BadAlignment);
		}

		#[cfg(target_pointer_width = "32")]
		let class = 1;
		#[cfg(target_pointer_width = "64")]
		let class = 2;
		if identifier.class != class {
			return Err(ParseError::UnsupportedClass);
		}

		#[cfg(target_endian = "little")]
		let endian = 1;
		#[cfg(target_endian = "big")]
		let endian = 2;
		if identifier.data != endian {
			return Err(ParseError::UnsupportedData);
		}

		if identifier.version != 1 {
			return Err(ParseError::UnsupportedVersion);
		}

		if data.len() < mem::size_of::<FileHeader>() {
			return Err(ParseError::HeaderTooSmall);
		}
		// SAFETY: the data is long enough
		let header = unsafe { &*(data as *const [u8] as *const FileHeader) };

		if header.typ != TYPE_EXEC {
			return Err(ParseError::UnsupportedType);
		}

		if header.machine != arch::ELF_MACHINE {
			return Err(ParseError::UnsupportedMachine);
		}

		if header.flags & !arch::ELF_FLAGS > 0 {
			return Err(ParseError::UnsupportedFlags);
		}

		// Parse the program headers and create the segments.

		let count = header.program_header_entry_count as usize;
		let size = header.program_header_entry_size as usize;
		if size != mem::size_of::<ProgramHeader>() {
			return Err(ParseError::BadProgramHeaderSize);
		}
		if data.len() < count * size + header.program_header_offset {
			return Err(ParseError::ProgramHeadersLargerThanFile);
		}

		let mut segments =
			Box::try_new_uninit_slice_in(count, allocator).map_err(ParseError::AllocError)?;
		for i in 0..count {
			// SAFETY: the data is large enough and aligned and the header size matches.
			let header = unsafe {
				&*((data as *const [u8] as *const u8).add(header.program_header_offset)
					as *const ProgramHeader)
			};
			let mut order = 0;
			let mut align = header.alignment / arch::PAGE_SIZE;
			// naive integer log2
			while align > 0 {
				order += 1;
				align >>= 1;
			}
			let page = MEMORY_MANAGER
				.lock()
				.allocate(order as usize)
				.map_err(ParseError::AllocateError)?;
			// FIXME can panic if the header is bad
			let data = data[header.offset..][..header.file_size].as_ptr();
			// SAFETY: FIXME
			unsafe { ptr::copy_nonoverlapping(data, page.cast().as_ptr(), header.file_size) };
			segments[i].write(Segment {
				flags: header.flags as u8,
				order,
				page,
				virtual_address: header.virtual_address,
			});
		}

		// SAFETY: all segments are initialized.
		let segments = unsafe { segments.assume_init() };

		Ok(Self {
			entry: header.entry,
			segments,
		})
	}

	/// Returns the physical entry address.
	pub fn physical_entry(&self) -> *const () {
		let address = self.entry - self.segments[0].virtual_address;
		unsafe { self.segments[0].page.cast::<u8>().as_ptr().add(address).cast() }
	}

	/// Starts executing the binary by jumping to the start of the first segment
	///
	/// ## Safety
	///
	/// Since the binary may contain any arbitrary instructions it is up to the caller to ensure
	/// adequate protections have been set up.
	pub unsafe fn execute(&self) {
		let address = self.physical_entry();
		let func: unsafe extern "C" fn() = unsafe { mem::transmute(address) };
		func();
	}

	/// Creates a new task from the ELF data.
	pub fn create_task(&self) -> Result<crate::task::Task, crate::memory::AllocateError> {
		let mut task = crate::task::Task::new()?;
		for s in self.segments.iter() {
			task.add_mapping(s.page, s.order);
		}
		task.set_pc(self.physical_entry());
		Ok(task)
	}
}

impl ProgramHeader {
	const FLAG_EXEC: u32 = 0x1;
	const FLAG_WRITE: u32 = 0x2;
	const FLAG_READ: u32 = 0x4;
}

impl Segment {
	const FLAG_EXEC: u32 = 0x1;
	const FLAG_WRITE: u32 = 0x2;
	const FLAG_READ: u32 = 0x4;
}

impl Drop for Segment {
	fn drop(&mut self) {
		unsafe {
			// SAFETY: we own the page and nothing else is using the memory (if something was, we
			// shouldn't be being dropped in the first place).
			if MEMORY_MANAGER
				.lock()
				.deallocate(self.page, self.order as usize)
				.is_err()
			{
				log::error(&[
					file!(),
					":",
					concat!(line!()),
					" Failed to deallocate memory page",
				]);
			}
		}
	}
}

#[cfg(test)]
//impl test { // TODO this caused an ICE. Reproduce and report this.
mod test {
	use super::*;
	use crate::{log, task};

	const HELLO_WORLD_ELF_RISCV64: &[u8] =
		include_bytes!("../../services/init/hello_world/build/init");

	test!(parse_hello_world() {
		let heap = MEMORY_MANAGER.lock().allocate(3).unwrap();
		let heap = unsafe { crate::alloc::allocators::WaterMark::new(heap.cast(), 4096) };
		let elf = ELF::parse(HELLO_WORLD_ELF_RISCV64, heap).unwrap();
		let mut task_a = task::Task::new().unwrap();
		let mut task_b = task::Task::new().unwrap();
		for s in elf.segments.iter() {
			log::debug_usize("segment flags", s.flags as usize, 2);
			log::debug_usize("segment order", s.order as usize, 10);
			task_a.add_mapping(s.page, s.order);
			task_b.add_mapping(s.page, s.order);
		}
		task_a.set_pc(elf.physical_entry());
		task_b.set_pc(elf.physical_entry());
		task_a.insert(task_b);
		log::debug_str("Executing...");
		task_a.next();
		log::debug_str("Finished");
	});
}
