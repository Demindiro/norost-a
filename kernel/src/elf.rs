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

use crate::arch;
use crate::memory::PPNRange;
use core::convert::TryInto;
use core::mem;
use core::ptr::NonNull;

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

const TYPE_EXEC: u16 = 2;

pub struct Segment {
	/// The address to map the segment to.
	pub address: arch::Page,
	/// The PPNs of this segment.
	pub ppn: PPNRange,
	/// The RWX flags.
	pub flags: arch::vms::RWX,
}

/// Parse the ELF file and set the PPNs & flags to be mapped.
///
/// ## Panics
///
/// The ELF file has bad data anywhere. Panicking is fine since if the init ELF cannot
/// be parsed we cannot continue anyways.
pub fn parse(data: &[u8], segments: &mut [Option<Segment>], entry: &mut *const ()) {
	// Parse the file header

	assert!(data.len() >= 16, "Data too short to include magic");

	// SAFETY: the data is at least 16 bytes long
	let identifier = unsafe { &*(data as *const [u8] as *const Identifier) };

	assert_eq!(&identifier.magic, b"\x7fELF", "Bad ELF magic");
	assert_eq!(
		data.as_ptr().align_offset(mem::size_of::<usize>()),
		0,
		"Bad alignment"
	);

	#[cfg(target_pointer_width = "32")]
	assert_eq!(identifier.class, 1, "Unsupported class");
	#[cfg(target_pointer_width = "64")]
	assert_eq!(identifier.class, 2, "Unsupported class");

	#[cfg(target_endian = "little")]
	assert_eq!(identifier.data, 1, "Unsupported endianness");
	#[cfg(target_endian = "big")]
	assert_eq!(identifier.data, 2, "Unsupported endianness");

	assert_eq!(identifier.version, 1, "Unsupported version");

	assert!(
		data.len() >= mem::size_of::<FileHeader>(),
		"Header too small"
	);
	// SAFETY: the data is long enough
	let header = unsafe { &*(data as *const [u8] as *const FileHeader) };

	assert_eq!(header.typ, TYPE_EXEC, "Unsupported type");

	assert_eq!(
		header.machine,
		arch::ELF_MACHINE,
		"Unsupported machine type"
	);

	assert_eq!(header.flags & !arch::ELF_FLAGS, 0, "Unsupported flags");

	// Parse the program headers and create the segments.

	let count = header.program_header_entry_count as usize;
	let size = header.program_header_entry_size as usize;
	assert_eq!(
		size,
		mem::size_of::<ProgramHeader>(),
		"Bad program header size"
	);
	assert!(
		data.len() >= count * size + header.program_header_offset,
		"Program headers exceed the size of the file"
	);

	let mut i = 0;
	for k in 0..count {
		assert!(i < segments.len(), "Too many segments");

		// SAFETY: the data is large enough and aligned and the header size matches.
		let header = unsafe {
			let h = data as *const [u8] as *const u8;
			let h = h.add(header.program_header_offset);
			let h = h as *const ProgramHeader;
			&*h.add(k)
		};

		// Skip non-loadable segments
		if header.typ != ProgramHeader::TYPE_LOAD {
			continue;
		}

		use arch::vms::RWX;

		let flags = match header.flags & 7 {
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

		assert_eq!(
			header.offset & arch::PAGE_MASK,
			header.virtual_address & arch::PAGE_MASK,
			"Offset is not aligned"
		);

		let offset = header.offset & arch::PAGE_MASK;

		let address = ((header.virtual_address & !arch::PAGE_MASK) as *mut ())
			.try_into()
			.expect("Address is 0x0");
		let count = ((header.memory_size + offset + arch::PAGE_MASK) / arch::Page::SIZE)
			.try_into()
			.expect("Segment too large");
		// TODO add 'register' method to PMM that marks a page as managed by PMM but already
		// allocated.
		// FIXME these pages may be shared.
		let ppn = &data[header.offset & !arch::PAGE_MASK..] as *const _ as *const u8 as usize;
		let ppn = unsafe { PPNRange::from_ptr(ppn, count) };

		dbg!(address, &ppn, flags);
		segments[i] = Some(Segment {
			address,
			ppn,
			flags,
		});
		i += 1;
	}

	*entry = header.entry as *const _;
}

const FLAG_EXEC: u32 = 0x1;
const FLAG_WRITE: u32 = 0x2;
const FLAG_READ: u32 = 0x4;

impl ProgramHeader {
	const TYPE_LOAD: u32 = 1;
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
