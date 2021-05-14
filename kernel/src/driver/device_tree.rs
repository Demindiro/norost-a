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

// TODO we should add range checks just in case a manufacturer screws up their DTB.

use crate::util::{self, BigEndianU32, BigEndianU64};
use core::slice;

/// A structure representing a device tree.
pub struct DeviceTree {
	/// The header of the DTB
	header: &'static Header,
}

/// An enum representing possible errors that can occur while parsing
/// a DTB.
#[derive(Debug)]
pub enum ParseDTBError {
	/// The magic doesn't match (i.e. it isn't `0xdOOdfeed`)
	BadMagic(u32),
	/// The version is unsupported (i.e. it isn't either `16` or `17`)
	IncompatibleVersion(u32),
}

/// A representation of the header field of the DTB format.
#[repr(C)]
struct Header {
	/// A field that must contain `0xdOOdfeed`.
	magic: BigEndianU32,
	/// The total size of the DTB in bytes.
	total_size: BigEndianU32,
	/// The offset of the [`StructureBlock`] from the beginning of the header in bytes.
	offset_structure_block: BigEndianU32,
	/// The offset of the [`StringsBlock´] from the beginning of the header in bytes.
	offset_strings_block: BigEndianU32,
	/// The offset of the [`MemoryReservationBlock`] from the beginning of the header in bytes.
	offset_memory_reservation_block: BigEndianU32,
	/// The version of the DTB structure. Must be `17`.
	version: BigEndianU32,
	/// The lowest version with which this DTB structure is backwards compatible. Must be `16`.
	last_compatible_version: BigEndianU32,
	/// The physical ID of the boot CPU.
	boot_cpu_id_physical: BigEndianU32,
	/// The size of the [`StringsBlock`] in bytes.
	size_strings_block: BigEndianU32,
	/// The size of the [`StructureBlock`] in bytes.
	size_structure_block: BigEndianU32,
}

/// A structure indicating a reserved memory region entry.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct ReservedMemoryRegion {
	pub address: BigEndianU64,
	pub size: BigEndianU64,
}

/// A structure representing the "structure block" of the DTB.
struct StructureBlock {
	_data: [u8; 0],
}

/// A structure representing the "strings block" of the DTB.
struct StringsBlock {
	_data: [u8; 0],
}

/// An interpreter to parse the tree inside the [`StructureBlock`].
pub struct Interpreter {
	/// The current pointer in the StructureBlock that is being parsed.
	pc: *const BigEndianU32,
	/// The address of the [`StringsBlock`]
	strings: &'static StringsBlock,
	/// Flag indicating whether the interpreter has not yet started, is running or has ended.
	state: InterpreterState,
}

/// Enum indicating whether the interpreter is fresh, has started or has ended.
enum InterpreterState {
	NotStarted,
	Running,
	Ended,
}

/// A structure representing a single node in the DTB
pub struct Node<'interpreter> {
	/// The state of the interpreter
	interpreter: &'interpreter mut Interpreter,
	/// The name of the node
	pub name: &'static str,
	/// The iteration state of this node.
	state: NodeState,
}

/// Enum indicating the enumeration state of a Node
enum NodeState {
	/// There are still properties left to iterate
	Properties,
	/// There are still child nodes left to iterate
	ChildNodes,
	/// There is nothing left to iterate
	Empty,
}

/// A structure representing a property of a node in the DTB
pub struct Property {
	/// The value of the property.
	pub value: &'static [u8],
	/// The name of the property.
	pub name: &'static str,
}

impl DeviceTree {
	/// The magic value that must be present in every valid DTB.
	const MAGIC: u32 = 0xd00dfeed;

	/// Parse the DTB located at the given address.
	///
	/// ## Safety
	///
	/// The address must be valid and may never be deallocated.
	// TODO change *const u8 to NonNull
	pub unsafe fn parse_dtb(address: *const u8) -> Result<Self, ParseDTBError> {
		let header = &*(address as *const Header);

		if header.magic.get() != Self::MAGIC {
			return Err(ParseDTBError::BadMagic(header.magic.get()));
		}

		Ok(Self { header })
	}

	/// The physical ID of the boot CPU.
	pub fn boot_cpu_id(&self) -> u32 {
		self.header.boot_cpu_id_physical.get()
	}

	/// A iterator over all reserved memory regions.
	pub fn reserved_memory_regions(&self) -> impl Iterator<Item = ReservedMemoryRegion> {
		struct Iter {
			rmr: *const ReservedMemoryRegion,
		}

		impl Iterator for Iter {
			type Item = ReservedMemoryRegion;

			fn next(&mut self) -> Option<Self::Item> {
				// SAFETY: The DTB is valid
				let rmr = unsafe { *self.rmr };
				if rmr.address.get() == 0 && rmr.size.get() == 0 {
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
			(self.header as *const _ as *const u8)
				.add(self.header.offset_memory_reservation_block.get() as usize)
				.cast()
		};

		Iter { rmr }
	}

	/// Returns a new interpreter
	pub fn interpreter(&self) -> Interpreter {
		// SAFETY: The DTB is valid
		let pc = unsafe {
			(self.header as *const _ as *const u8)
				.add(self.header.offset_structure_block.get() as usize)
				.cast()
		};
		// SAFETY: The DTB is valid
		let strings = unsafe {
			&*(self.header as *const _ as *const u8)
				.add(self.header.offset_strings_block.get() as usize)
				.cast()
		};
		let state = InterpreterState::NotStarted;
		Interpreter { pc, strings, state }
	}
}

impl StringsBlock {
	/// Returns the string at the given offset
	fn get<'a>(&'a self, offset: u32) -> &'a str {
		// SAFETY: The offset is in range.
		unsafe {
			let ptr = (self as *const _ as *const u8).add(offset as usize);
			util::cstr_to_str(ptr).expect("String isn't valid UTF-8")
		}
	}
}

impl Interpreter {
	const TOKEN_BEGIN_NODE: u32 = 0x1;
	const TOKEN_END_NODE: u32 = 0x2;
	const TOKEN_PROP: u32 = 0x3;
	const TOKEN_NOP: u32 = 0x4;
	const TOKEN_END: u32 = 0x9;

	/// Returns the next node
	pub fn next_node(&mut self) -> Option<Node> {
		self.step_node()
	}

	/// Return the current token
	fn current(&self) -> u32 {
		// SAFETY: the token tree is valid and the pointer is aligned
		unsafe { *self.pc }.get()
	}

	/// Advances the program counter by the given number of steps
	// TODO check if the PC is still in range
	fn advance(&mut self, steps: u32) {
		// SAFETY: the pointer remains within the structure block
		// or right at the end of it.
		self.pc = unsafe { self.pc.add(steps as usize) };
	}

	/// Return the current token and advance the program counter
	fn step(&mut self) -> u32 {
		let tk = self.current();
		self.advance(1);
		tk
	}

	/// Rewinds the program counter by the given number of steps
	fn rewind(&mut self, steps: u32) {
		// SAFETY: the pointer remains within the structure block
		// or right at the end of it.
		self.pc = unsafe { self.pc.sub(steps as usize) };
	}

	/// Skip `TOKEN_NOP` until `TOKEN_BEGIN_NODE` is encountered, then return the [`Node`].
	///
	/// Returns `None` if `TOKEN_END` is encountered.
	///
	/// ## Panics
	///
	/// `TOKEN_PROP` or `TOKEN_END_NODE` is encountered.
	fn step_node(&mut self) -> Option<Node> {
		// SAFETY: the token tree is valid and the pointer is aligned
		loop {
			match self.step() {
				Self::TOKEN_BEGIN_NODE => {
					let ptr = self.pc as *const u8;
					// SAFETY: ptr points to a null-terminated byte string
					let name = unsafe { util::cstr_to_str(ptr).expect("Invalid UTF-8 node name") };
					let align = name.len() as u32 + 1; // Include null terminator
					let align = (align + 3) & !3;
					self.advance(align / 4);
					break Some(Node {
						interpreter: self,
						name,
						state: NodeState::Properties,
					});
				}
				Self::TOKEN_END_NODE => {
					break None;
				}
				Self::TOKEN_PROP => panic!("Unexpected TOKEN_PROP"),
				Self::TOKEN_NOP => (),
				Self::TOKEN_END => {
					self.state = InterpreterState::Ended;
					break None;
				}
				_ => panic!("Invalid token in DTB"),
			}
		}
	}

	/// Skip `TOKEN_NOP` until `TOKEN_PROP` is encountered, then return the [`Property`].
	///
	/// Returns `None` if `TOKEN_END` or `TOKEN_BEGIN_NODE` is encountered.
	///
	/// ## Panics
	///
	/// `TOKEN_END_NODE` is encountered.
	fn step_property(&mut self) -> Option<Property> {
		loop {
			match self.step() {
				Self::TOKEN_BEGIN_NODE => {
					self.rewind(1);
					break None;
				}
				Self::TOKEN_END_NODE => {
					// next_node() will consume the token
					self.rewind(1);
					break None;
				}
				Self::TOKEN_PROP => {
					let len = self.step();
					let name = self.step();
					let name = self.strings.get(name);
					let ptr = self.pc as *const u8;
					// SAFETY: the pointer and length are valid.
					let value = unsafe { slice::from_raw_parts(ptr, len as usize) };
					let align = (len + 3) & !3;
					self.advance(align / 4);
					break Some(Property { name, value });
				}
				Self::TOKEN_NOP => (),
				Self::TOKEN_END => {
					self.state = InterpreterState::Ended;
					break None;
				}
				_tk => panic!("Invalid token in DTB"),
			}
		}
	}
}

impl Node<'_> {
	/// Returns the next property of this node.
	pub fn next_property(&mut self) -> Option<Property> {
		if let NodeState::Properties = self.state {
			if let Some(p) = self.interpreter.step_property() {
				Some(p)
			} else {
				self.state = NodeState::ChildNodes;
				None
			}
		} else {
			None
		}
	}

	/// Returns the next child node of this node.
	pub fn next_child_node(&mut self) -> Option<Node> {
		if let NodeState::ChildNodes = self.state {
			if let Some(n) = self.interpreter.step_node() {
				Some(n)
			} else {
				self.state = NodeState::Empty;
				None
			}
		} else {
			None
		}
	}
}

impl Drop for Node<'_> {
	/// Ensure that the interpreter skips any unread fields of this node.
	fn drop(&mut self) {
		while self.next_property().is_some() {}
		while self.next_child_node().is_some() {}
	}
}
