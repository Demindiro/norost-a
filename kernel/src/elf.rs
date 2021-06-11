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
use crate::arch;
use crate::memory;
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
	/// The virtual address of the start of this segment.
	virtual_area: NonNull<arch::Page>,
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

/// Parse the ELF file and return a new task.
pub fn create_task(data: &[u8]) -> crate::task::Task {
	// Parse the file header

	if data.len() < 16 {
		panic!("Data too short to include magic");
	}

	// SAFETY: the data is at least 16 bytes long
	let identifier = unsafe { &*(data as *const [u8] as *const Identifier) };

	if &identifier.magic != b"\x7fELF" {
		panic!("Bad ELF magic");
	}

	if data.as_ptr().align_offset(mem::size_of::<usize>()) != 0 {
		panic!("Bad alignment");
	}

	#[cfg(target_pointer_width = "32")]
	let class = 1;
	#[cfg(target_pointer_width = "64")]
	let class = 2;
	if identifier.class != class {
		panic!("Unsupported class");
	}

	#[cfg(target_endian = "little")]
	let endian = 1;
	#[cfg(target_endian = "big")]
	let endian = 2;
	if identifier.data != endian {
		panic!("Unsupported endianness");
	}

	if identifier.version != 1 {
		panic!("Unsupported version");
	}

	if data.len() < mem::size_of::<FileHeader>() {
		panic!("Header too small");
	}
	// SAFETY: the data is long enough
	let header = unsafe { &*(data as *const [u8] as *const FileHeader) };

	if header.typ != TYPE_EXEC {
		panic!("Unsupported type");
	}

	if header.machine != arch::ELF_MACHINE {
		panic!("Unsupported machine type");
	}

	if header.flags & !arch::ELF_FLAGS > 0 {
		panic!("Unsupported flags");
	}

	// Parse the program headers and create the segments.

	let count = header.program_header_entry_count as usize;
	let size = header.program_header_entry_size as usize;
	if size != mem::size_of::<ProgramHeader>() {
		panic!("Bad program header size");
	}
	if data.len() < count * size + header.program_header_offset {
		panic!("Program headers exceed the size of the file");
	}

	// Count the amount of loadable segments.
	let mut loadable_count = 0;
	for i in 0..count {
		// SAFETY: the data is large enough and aligned and the header size matches.
		let header = unsafe {
			let h = data as *const [u8] as *const u8;
			let h = h.add(header.program_header_offset);
			let h = h as *const ProgramHeader;
			&*h.add(i)
		};
		if header.typ == ProgramHeader::TYPE_LOAD {
			loadable_count += 1;
		}
	}

	let mut task = crate::task::Task::new().expect("Failed to allocate task");

	for i in 0..count {
		// SAFETY: the data is large enough and aligned and the header size matches.
		let header = unsafe {
			let h = data as *const [u8] as *const u8;
			let h = h.add(header.program_header_offset);
			let h = h as *const ProgramHeader;
			&*h.add(i)
		};

		// Skip non-loadable segments
		if header.typ != ProgramHeader::TYPE_LOAD {
			continue;
		}

		use arch::RWX;

		// Set flags
		let rwx = match header.flags & 7 {
			f if f == FLAG_EXEC | FLAG_WRITE | FLAG_READ => RWX::RWX,
			f if f == FLAG_EXEC | FLAG_READ => RWX::RX,
			f if f == FLAG_EXEC | FLAG_WRITE => panic!("Write-execute pages are unsupported"),
			f if f == FLAG_EXEC => RWX::X,
			f if f == FLAG_WRITE | FLAG_READ => RWX::RW,
			f if f == FLAG_WRITE => panic!("Write-only pages are unsupported"),
			f if f == FLAG_READ => RWX::R,
			f if f == 0 => panic!("Flagless pages are unsupported"),
			_ => unreachable!(),
		};

		assert_eq!(header.offset & arch::PAGE_MASK, 0, "Offset is not aligned");
		let from = NonNull::from(&data[header.offset..][..header.file_size]).cast();
		let to = NonNull::new(header.virtual_address as *mut _).unwrap();
		arch::VirtualMemorySystem::alias_address(from, to, rwx, true, false).unwrap();
	}

	task.set_pc(header.entry as *const _);

	task
}

const FLAG_EXEC: u32 = 0x1;
const FLAG_WRITE: u32 = 0x2;
const FLAG_READ: u32 = 0x4;

impl ProgramHeader {
	const TYPE_LOAD: u32 = 1;
}

impl Segment {
	const FLAG_EXEC: u32 = 0x1;
	const FLAG_WRITE: u32 = 0x2;
	const FLAG_READ: u32 = 0x4;
}

#[cfg(test)]
//impl test { // TODO this caused an ICE. Reproduce and report this.
mod test {
	use super::*;
	use crate::{log, task};

	const HELLO_WORLD_ELF_RISCV64: &[u8] =
		include_bytes!("../../services/init/hello_world/build/init");

	/*
	test!(parse_hello_world() {
		let heap = memory::mem_allocate(3).unwrap();
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
	*/
}
