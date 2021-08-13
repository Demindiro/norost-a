//! Implementation of Device Trees.
//!
//! Device Trees are a way to detect any external hardware. Hardware layout is described with
//! DTB files, which are in turn created from DTS files.
//!
//! This module defines structures to parse **version 17** DTB files. By extension, this module can
//! parse **version 16** DTBs since version 17 is backwards compatible with it.
//!
//! ## References
//!
//! [Devicetree Specification 0.3][dt spec]
//!
//! [dt spec]: https://github.com/devicetree-org/devicetree-specification/releases/download/v0.3/devicetree-specification-v0.3.pdf

#![cfg_attr(not(test), no_std)]

use core::convert::{TryFrom, TryInto};
use core::fmt;
use core::mem;
use core::slice;
use simple_endian::{u32be, u64be};

/// A structure representing a device tree.
pub struct DeviceTree<'a> {
	data: &'a [u32],
}

/// An enum representing possible errors that can occur while parsing
/// a DTB.
#[derive(Debug)]
pub enum ParseError {
	/// There is too little data in the DTB to be possibly valid.
	TooShort,
	/// The magic doesn't match (i.e. it isn't `0xdOOdfeed`)
	BadMagic(u32),
}

#[derive(Debug)]
pub enum ParseNodeError {
	/// A different interpreter token was expected
	UnexpectedToken,
	/// The name was not null-terminated.
	UnterminatedName,
	/// An offset was out of bounds.
	OutOfBounds,
	/// The DTB is too short / truncated.
	TooShort,
	/// The value of a `#...-cells` isn't 32 bits large.
	BadCellsValue,
}

/// A representation of the header field of the DTB format.
#[repr(C)]
struct Header {
	/// A field that must contain `0xdOOdfeed`.
	magic: u32be,
	/// The total size of the DTB in bytes.
	total_size: u32be,
	/// The offset of the [`StructureBlock`] from the beginning of the header in bytes.
	offset_structure_block: u32be,
	/// The offset of the [`StringsBlock´] from the beginning of the header in bytes.
	offset_strings_block: u32be,
	/// The offset of the [`MemoryReservationBlock`] from the beginning of the header in bytes.
	offset_memory_reservation_block: u32be,
	/// The version of the DTB structure. Must be `17`.
	version: u32be,
	/// The lowest version with which this DTB structure is backwards compatible. Must be `16`.
	last_compatible_version: u32be,
	/// The physical ID of the boot CPU.
	boot_cpu_id_physical: u32be,
	/// The size of the [`StringsBlock`] in bytes.
	size_strings_block: u32be,
	/// The size of the [`StructureBlock`] in bytes.
	size_structure_block: u32be,
}

/// A structure indicating a reserved memory region entry.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct ReservedMemoryRegion {
	pub address: u64be,
	pub size: u64be,
}

/// A structure representing the "strings block" of the DTB.
struct StringsBlock<'a> {
	data: &'a [u8],
}

/// A structure representing a single node in the DTB
pub struct Node<'a, 'b: 'a> {
	/// The DTB that is being parsed.
	dtb: &'a DeviceTree<'b>,
	/// The name of the node.
	pub name: &'b [u8],
	/// The value of `#address-cells` at this level.
	pub address_cells: u32,
	/// The value of `#size-cells` at this level.
	pub size_cells: u32,
	/// The value of `#interrupt-cells` at this level.
	pub interrupt_cells: u32,
	/// An index to the start of the properties list.
	properties: u32,
	/// An index to the start of the nodes list.
	children: u32,
}

/// A structure representing a property of a node in the DTB
pub struct Property<'a> {
	/// The value of the property.
	pub value: &'a [u8],
	/// The name of the property.
	pub name: &'a [u8],
}

impl<'a> DeviceTree<'a> {
	/// The magic value that must be present in every valid DTB.
	const MAGIC: u32 = 0xd00dfeed;

	/// Parse the DTB data.
	pub fn parse(data: &'a [u32]) -> Result<Self, ParseError> {
		let byte_len = data.len() * mem::size_of::<u32>();
		(byte_len >= mem::size_of::<Header>())
			.then(|| ())
			.ok_or(ParseError::TooShort)?;

		// SAFETY: the address is properly aligned & large enough to fit the entire header.
		let header = unsafe { &*(data as *const _ as *const Header) };

		(byte_len >= u32::from(header.total_size).try_into().unwrap())
			.then(|| ())
			.ok_or(ParseError::TooShort)?;

		(header.magic == Self::MAGIC.into())
			.then(|| ())
			.ok_or(ParseError::BadMagic(header.magic.into()))?;

		Ok(Self { data })
	}

	/// A iterator over all reserved memory regions.
	// TODO there is also a "reserved-memory" node that we currently use. It seems the
	// information in that node is not reflected in the memory reservations block. Can we
	// remove this function or not?
	pub fn reserved_memory_regions(&self) -> impl Iterator<Item = ReservedMemoryRegion> {
		struct Iter {
			rmr: *const ReservedMemoryRegion,
		}

		impl Iterator for Iter {
			type Item = ReservedMemoryRegion;

			fn next(&mut self) -> Option<Self::Item> {
				// SAFETY: The DTB is valid
				let rmr = unsafe { *self.rmr };
				if rmr.address == 0.into() && rmr.size == 0.into() {
					None
				} else {
					// SAFETY: The DTB is valid
					self.rmr = unsafe { self.rmr.add(1) };
					Some(rmr)
				}
			}
		}

		// SAFETY: The DTB is valid
		let rmr = unsafe {
			(self.data as *const _ as *const u8)
				.add(u32::from(self.header().offset_memory_reservation_block) as usize)
				.cast()
		};

		Iter { rmr }
	}

	/// Return the root node.
	pub fn root(&self) -> Result<Node, ParseNodeError> {
		Node::new(
			self,
			u32::from(self.header().offset_structure_block)
				/ u32::try_from(mem::size_of::<u32>()).unwrap(),
			2, // If missing, we should assume 2 for address-cells.
			1, // Ditto
			0, // No idea about this one. 0 seems like a sane default?
		)
		.map(|n| n.0)
	}

	/// Return the total size of the FDT
	pub fn total_size(&self) -> usize {
		u32::from(self.header().total_size) as usize
	}

	/// Return a reference to the strings block
	fn strings(&self) -> StringsBlock<'a> {
		let h = self.header();
		// SAFETY: The DTB is valid
		let data = unsafe {
			let ptr = (self.data as *const _ as *const u8)
				.add(u32::from(h.offset_strings_block).try_into().unwrap());
			slice::from_raw_parts(ptr, u32::from(h.size_strings_block).try_into().unwrap())
		};
		StringsBlock { data }
	}

	/// Return a reference to the header
	fn header(&self) -> &'a Header {
		// SAFETY: the DTB is valid and properly aligned.
		unsafe { &*(self.data as *const [u32] as *const Header) }
	}

	/// Return an `u32` at the given position
	fn get(&self, position: u32) -> Option<u32> {
		self.data
			.get(usize::try_from(position).unwrap())
			.copied()
			.map(u32::from_be)
	}
}

impl fmt::Debug for DeviceTree<'_> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		// TODO print reserved memory areas too
		self.root().fmt(f)
	}
}

impl<'a> StringsBlock<'a> {
	/// Returns the string at the given offset
	fn get(&self, offset: u32) -> Option<&'a [u8]> {
		self.data
			.get(offset.try_into().unwrap()..)
			.map(cstr_to_str)
			.flatten()
	}
}

impl<'a, 'b> Node<'a, 'b> {
	const TOKEN_BEGIN_NODE: u32 = 0x1;
	const TOKEN_END_NODE: u32 = 0x2;
	const TOKEN_PROP: u32 = 0x3;
	const TOKEN_NOP: u32 = 0x4;
	const TOKEN_END: u32 = 0x9;

	/// Parse a node in a tree.
	fn new(
		dtb: &'a DeviceTree<'b>,
		mut offset: u32,
		address_cells: u32,
		size_cells: u32,
		interrupt_cells: u32,
	) -> Result<(Self, u32), ParseNodeError> {
		// Ensure this is indeed the start of a node
		(dtb.get(offset) == Some(Self::TOKEN_BEGIN_NODE))
			.then(|| ())
			.ok_or(ParseNodeError::UnexpectedToken)?;
		offset += 1;

		// Parse name
		let name = cstr_to_str(&dtb.data[offset.try_into().unwrap()..])
			.ok_or(ParseNodeError::UnterminatedName)?;
		let align = name.len() + 1; // Include null terminator
		let mask = mem::align_of::<u32>() - 1;
		offset += u32::try_from((align + mask) / mem::size_of::<u32>()).unwrap();

		let properties = offset;
		// Parse properties
		Self::is_token_valid(dtb, offset).unwrap();

		while dtb.get(offset) == Some(Self::TOKEN_PROP) {
			offset += 1;

			let len = dtb.get(offset).ok_or(ParseNodeError::TooShort)?;
			offset += 1;

			let name = dtb.get(offset).ok_or(ParseNodeError::TooShort)?;
			let name = dtb.strings().get(name).ok_or(ParseNodeError::OutOfBounds)?;
			offset += 1;

			let size = u32::try_from(mem::align_of::<u32>()).unwrap();

			match name {
				b"#address-cells" | b"#size-cells" | b"#interrupt-cells" => (size == 4)
					.then(|| ())
					.ok_or(ParseNodeError::BadCellsValue)?,
				_ => (),
			}

			offset += (len + size - 1) / size;
		}

		let children = offset;
		while Self::TOKEN_END_NODE != dtb.get(offset).ok_or(ParseNodeError::TooShort)? {
			// The size of the cells doesn't matter right now, so just pass 0
			let (_, offt) = Self::new(dtb, offset, 0, 0, 0)?;
			offset = offt;
		}

		(dtb.get(offset) == Some(Self::TOKEN_END_NODE))
			.then(|| ())
			.ok_or(ParseNodeError::UnexpectedToken)?;
		offset += 1;

		Ok((
			Self {
				dtb,
				name,
				properties,
				children,
				address_cells,
				size_cells,
				interrupt_cells,
			},
			offset,
		))
	}

	/// Return an iterator over all the properties of this node
	pub fn properties(&self) -> impl Iterator<Item = Property<'b>> + fmt::Debug + '_ {
		struct Iter<'a, 'b: 'a> {
			dtb: &'a DeviceTree<'b>,
			offset: u32,
		}

		impl<'a, 'b> Iterator for Iter<'a, 'b> {
			type Item = Property<'b>;

			fn next(&mut self) -> Option<Self::Item> {
				#[cfg(debug_assertions)]
				Node::is_token_valid(self.dtb, self.offset).expect("invalid token");
				(self.dtb.get(self.offset) == Some(Node::TOKEN_PROP)).then(|| {
					self.offset += 1;

					let len = self.dtb.get(self.offset).unwrap();
					self.offset += 1;

					let name = self.dtb.get(self.offset).unwrap();
					let name = self.dtb.strings().get(name).unwrap();
					self.offset += 1;

					let value = &self.dtb.data[self.offset.try_into().unwrap()..];
					let value = unsafe {
						slice::from_raw_parts(
							value as *const _ as *const _,
							value.len() * mem::size_of::<u32>(),
						)
					};
					let value = &value[..len.try_into().unwrap()];
					let size = u32::try_from(mem::align_of::<u32>()).unwrap();
					self.offset += (u32::try_from(len).unwrap() + size - 1) / size;

					Property { name, value }
				})
			}
		}

		impl fmt::Debug for Iter<'_, '_> {
			fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
				let &Iter { dtb, offset } = self;
				let iter = Self { dtb, offset };
				f.debug_list().entries(iter).finish()
			}
		}

		Iter {
			dtb: self.dtb,
			offset: self.properties,
		}
	}

	/// Return an iterator over all the children of this node
	pub fn children(&self) -> impl Iterator<Item = Node<'a, 'b>> + fmt::Debug + '_ {
		struct Iter<'a, 'b: 'a> {
			dtb: &'a DeviceTree<'b>,
			offset: u32,
			address_cells: u32,
			size_cells: u32,
			interrupt_cells: u32,
		}

		impl<'a, 'b> Iterator for Iter<'a, 'b> {
			type Item = Node<'a, 'b>;

			fn next(&mut self) -> Option<Self::Item> {
				#[cfg(debug_assertions)]
				Node::is_token_valid(self.dtb, self.offset).expect("invalid token");
				(self.dtb.get(self.offset) == Some(Node::TOKEN_BEGIN_NODE)).then(|| {
					let (node, offt) = Node::new(
						self.dtb,
						self.offset,
						self.address_cells,
						self.size_cells,
						self.interrupt_cells,
					)
					.unwrap();
					self.offset = offt;
					node
				})
			}
		}

		impl fmt::Debug for Iter<'_, '_> {
			fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
				let &Iter {
					dtb,
					offset,
					address_cells,
					size_cells,
					interrupt_cells,
				} = self;
				let iter = Self {
					dtb,
					offset,
					address_cells,
					size_cells,
					interrupt_cells,
				};
				f.debug_list().entries(iter).finish()
			}
		}

		// Properties the direct descendants inherit
		let (mut address_cells, mut size_cells, mut interrupt_cells) = (2, 1, 0);

		for p in self.properties() {
			match p.name {
				b"#address-cells" => {
					address_cells = u32::from_be_bytes(p.value.try_into().unwrap());
				}
				b"#size-cells" => {
					size_cells = u32::from_be_bytes(p.value.try_into().unwrap());
				}
				b"#interrupt-cells" => {
					interrupt_cells = u32::from_be_bytes(p.value.try_into().unwrap());
				}
				_ => (),
			}
		}

		Iter {
			dtb: self.dtb,
			offset: self.children,
			address_cells,
			size_cells,
			interrupt_cells,
		}
	}

	/// Checks if a token is valid.
	fn is_token_valid(dtb: &DeviceTree<'_>, offset: u32) -> Result<u32, Option<u32>> {
		dtb.get(offset)
			.map(|v| match v {
				Node::TOKEN_BEGIN_NODE
				| Node::TOKEN_END_NODE
				| Node::TOKEN_PROP
				| Node::TOKEN_NOP
				| Node::TOKEN_END => Ok(v),
				_ => Err(Some(v)),
			})
			.unwrap_or(Err(None))
	}
}

impl fmt::Debug for Node<'_, '_> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let mut map = f.debug_map();
		if let Ok(name) = core::str::from_utf8(self.name) {
			map.entry(&"name", &name);
		} else {
			map.entry(&"name", &self.name);
		}
		map.entry(&"address_cells", &self.address_cells);
		map.entry(&"size_cells", &self.size_cells);
		map.entry(&"interrupt_cells", &self.interrupt_cells);
		map.entry(&"properties", &self.properties());
		map.entry(&"children", &self.children());
		map.finish()
	}
}

impl fmt::Debug for Property<'_> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		if let Ok(name) = core::str::from_utf8(self.name) {
			if let Ok(value) = core::str::from_utf8(self.value) {
				write!(f, "{:?}: {:?}", name, value)
			} else {
				write!(f, "{:?}: {:?}", name, self.value)
			}
		} else {
			write!(f, "{:?}: {:?}", self.name, self.value)
		}
	}
}

/// Converts a null-terminated C string to a Rust `[u8]`.
fn cstr_to_str<T>(s: &[T]) -> Option<&[u8]> {
	let len = s.len() * mem::size_of::<T>();
	// SAFETY: the alignment & length are valid
	let s = unsafe { slice::from_raw_parts(s as *const _ as *const _, len) };
	s.iter().position(|c| *c == 0).map(|l| &s[..l])
}

#[cfg(test)]
mod test {

	use super::*;

	/// Structure used to trick include_bytes! into aligning the array properly.
	#[repr(align(4))]
	struct Align<const S: usize>([u8; S]);

	impl<const S: usize> Align<S> {
		fn as_u32(&self) -> &[u32] {
			assert_eq!(
				self.0.len() % mem::align_of::<u32>(),
				0,
				"Data is not a multiple of 4"
			);
			unsafe {
				slice::from_raw_parts(self.0.as_ptr().cast(), self.0.len() / mem::size_of::<u32>())
			}
		}
	}

	#[test]
	fn qemu_system_riscv64() {
		let data = Align(*include_bytes!("../test/qemu_system_riscv64.dtb"));
		DeviceTree::parse(data.as_u32()).unwrap().root().unwrap();
	}
}
