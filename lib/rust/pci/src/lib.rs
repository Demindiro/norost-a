//! Library for iterating and interacting with PCI and PCIe devices.
//!
//! ## References
//!
//! [PCI on OSDev wiki][osdev pci]
//!
//! [osdev pci]: https://wiki.osdev.org/PCI

#![no_std]
#![feature(ptr_metadata)]

use core::cell::Cell;
use core::mem;
use core::ptr::NonNull;
use simple_endian::{u16le, u32le};
use vcell::VolatileCell;

pub const BAR_IO_SPACE: u32 = 1;
pub const BAR_TYPE_MASK: u32 = 0x6;

/// Common header fields.
#[repr(C)]
pub struct HeaderCommon {
	vendor_id: VolatileCell<u16le>,
	device_id: VolatileCell<u16le>,

	pub command: VolatileCell<u16le>,
	pub status: VolatileCell<u16le>,

	revision_id: VolatileCell<u8>,
	prog_if: VolatileCell<u8>,
	subclass: VolatileCell<u8>,
	class_code: VolatileCell<u8>,

	cache_line_size: VolatileCell<u8>,
	latency_timer: VolatileCell<u8>,
	header_type: VolatileCell<u8>,
	bist: VolatileCell<u8>,
}

impl HeaderCommon {
	/// Flag used to enable MMIO
	pub const COMMAND_MMIO_MASK: u16 = 0x2;
	/// Flag used to toggle bus mastering.
	pub const COMMAND_BUS_MASTER_MASK: u16 = 0x4;

	/// Set the flags in the command register.
	pub fn set_command(&self, flags: u16) {
		self.command.set(flags.into());
	}
}

/// Header type 0x00
#[repr(C)]
pub struct Header0 {
	pub common: HeaderCommon,

	pub base_address: [VolatileCell<u32le>; 6],

	cardbus_cis_pointer: VolatileCell<u32le>,

	subsystem_vendor_id: VolatileCell<u16le>,
	subsystem_id: VolatileCell<u16le>,

	expansion_rom_base_address: VolatileCell<u32le>,

	capabilities_pointer: VolatileCell<u8>,

	_reserved: [u8; 7],

	interrupt_line: VolatileCell<u8>,
	interrupt_pin: VolatileCell<u8>,
	min_grant: VolatileCell<u8>,
	max_latency: VolatileCell<u8>,
}

impl Header0 {
	pub const BASE_ADDRESS_COUNT: u8 = 6;

	/// Return the capability structures attached to this header.
	pub fn capabilities<'a>(&'a self) -> CapabilityIter<'a> {
		unsafe {
			let next = (self as *const _ as *const u8).add(self.capabilities_pointer.get().into());
			let next = Some(NonNull::new_unchecked(next as *mut Capability).cast());
			CapabilityIter {
				_header: self,
				next,
			}
		}
	}

	pub fn base_address(&self, index: usize) -> u32 {
		self.base_address[usize::from(index)].get().into()
	}

	pub fn set_base_address(&self, index: usize, value: u32) {
		self.base_address[usize::from(index)].set(value.into());
	}

	pub fn set_command(&self, value: u16) {
		self.common.set_command(value);
	}
}

/// Header type 0x01 (PCI-to-PCI bridge)
#[repr(C)]
pub struct Header1 {
	common: HeaderCommon,

	base_address: [VolatileCell<u32le>; 2],

	primary_bus_number: VolatileCell<u8>,
	secondary_bus_number: VolatileCell<u8>,
	subordinate_bus_number: VolatileCell<u8>,
	secondary_latency_timer: VolatileCell<u8>,

	io_base: VolatileCell<u8>,
	io_limit: VolatileCell<u8>,
	secondary_status: VolatileCell<u16le>,

	memory_base: VolatileCell<u16le>,
	memory_limit: VolatileCell<u16le>,

	prefetchable_memory_base: VolatileCell<u16le>,
	prefetchable_memory_limit: VolatileCell<u16le>,

	prefetchable_base_upper_32_bits: VolatileCell<u32le>,
	prefetchable_limit_upper_32_bits: VolatileCell<u32le>,

	io_base_upper_16_bits: VolatileCell<u16le>,
	io_limit_upper_16_bits: VolatileCell<u16le>,

	capabilities_pointer: VolatileCell<u8>,

	_reserved: [u8; 3],

	expansion_rom_base_address: VolatileCell<u32le>,

	interrupt_line: VolatileCell<u8>,
	interrupt_pin: VolatileCell<u8>,
	bridge_control: VolatileCell<u16le>,
}

/// Enum of possible headers.
pub enum Header<'a> {
	H0(&'a Header0),
	H1(&'a Header1),
	Unknown(&'a HeaderCommon),
}

impl<'a> Header<'a> {
	pub fn common(&self) -> &'a HeaderCommon {
		match self {
			Self::H0(h) => &h.common,
			Self::H1(h) => &h.common,
			Self::Unknown(hc) => hc,
		}
	}

	pub fn header_type(&self) -> u8 {
		self.common().header_type.get()
	}
}

#[repr(C)]
pub struct Capability {
	id: VolatileCell<u8>,
	next: VolatileCell<u8>,
}

impl Capability {
	/// Return the capability ID.
	pub fn id(&self) -> u8 {
		self.id.get()
	}

	/// Return a reference to data that is located right after the capability header.
	///
	/// ## Safety
	///
	/// It is up to the caller to ensure that the data actually exists and won't go out of bounds.
	pub unsafe fn data<'a, T>(&'a self) -> &'a T {
		&*(self as *const _ as *const u8).add(mem::size_of::<Self>()).cast()
	}
}

pub struct CapabilityIter<'a> {
	_header: &'a Header0,
	next: Option<NonNull<Capability>>,
}

impl<'a> Iterator for CapabilityIter<'a> {
	type Item = &'a Capability;

	fn next(&mut self) -> Option<Self::Item> {
		self.next.map(|next| {
			unsafe {
				let cap = next.as_ref();
				let offset = usize::from(cap.next.get());
				self.next = if offset != 0 {
					let next = (next.as_ptr() as usize & !0xff) + offset;
					NonNull::new(next as *mut Capability)
				} else {
					None
				};
				cap
			}
		})
	}
}

/// Representation of a PCI MMIO area
pub struct PCI {
	/// The start of the area
	start: NonNull<kernel::Page>,
	/// The size of the area in bytes
	size: usize,
	/// MMIO ranges for use with base addresses
	mem: [Option<PhysicalMemory>; 8],
	/// Ugly hacky but working counter for MMIO bump allocator.
	alloc_counter: Cell<usize>,
}

impl PCI {
	/// Create a new PCI MMIO wrapper.
	///
	/// `start` and `size` refer to the PCI configuration sections while `mmio` refers to the
	/// areas that can be allocated for use with base addresses.
	///
	/// ## Safety
	///
	/// The range must map to a valid PCI MMIO area.
	pub unsafe fn new(start: NonNull<kernel::Page>, size: usize, mem: &[PhysicalMemory]) -> Self {
		let mut mm = [None; 8];
		for (i, m) in mem.iter().copied().enumerate() {
			mm[i] = Some(m);
		}
		let mem = mm;
		let alloc_counter = Cell::new(0);
		Self { start, size, mem, alloc_counter }
	}

	/// Returns an iterator over all the valid devices.
	pub fn iter<'a>(&'a self) -> IterPCI<'a> {
		IterPCI {
			pci: self,
			bus: 0,
		}
	}

	/// Return a reference to the configuration header for a function.
	///
	/// Returns `None` if `vendor_id == 0xffff`
	///
	/// ## Panics
	///
	/// If the bus + device + function are out of the MMIO range.
	pub fn get(&self, bus: u8, device: u8, function: u8) -> Option<Header> {
		let h = self.get_unchecked(bus, device, function);
		if h.common().vendor_id.get() == 0xffff.into() {
			None
		} else {
			Some(h)
		}
	}

	/// Return a reference to the configuration header for a function. This won't
	/// return `None`, but the header values may be all `1`s.
	///
	/// ## Panics
	///
	/// If the bus + device + function are out of the MMIO range.
	fn get_unchecked<'a>(&'a self, bus: u8, device: u8, function: u8) -> Header<'a> {
		let offt = ((bus as usize) << 20) | ((device as usize) << 15) | ((function as usize) << 12);
		assert!(offt < self.size);
		unsafe {
			let h = self.start.as_ptr().cast::<u8>().add(offt);
			let hc = &*h.cast::<HeaderCommon>();
			match hc.header_type.get() & 0x7f {
				0 => Header::H0(&*h.cast()),
				1 => Header::H1(&*h.cast()),
				_ => Header::Unknown(hc),
			}
		}
	}

	/// Return a region of MMIO.
	///
	/// ## Notes
	///
	/// Currently all memory will be 16K byte aligned. Higher granulity will be supported later.
	pub fn allocate_mmio(&self, size: usize, flags: u8) -> Result<MMIO<'_>, ()> {
		assert!(size <= 1 << 16, "TODO");
		let size = 1 << 16;
		let c = self.alloc_counter.get();
		self.alloc_counter.set(c + size);
		Ok(MMIO {
			physical: self.mem[0].unwrap().physical + c,
			virt: NonNull::new(self.mem[0].unwrap().virt.as_ptr().wrapping_add(c)).unwrap().cast(),
			size,
			_pci: self,
		})
	}
}

/// A physically contiguous memory region.
#[derive(Clone, Copy)]
pub struct PhysicalMemory {
	/// The physical address
	pub physical: usize,
	/// The virtual address
	pub virt: NonNull<kernel::Page>,
	/// The size in bytes
	pub size: usize,
}

/// A MMIO region
pub struct MMIO<'a> {
	/// The physical address
	pub physical: usize,
	/// The virtual address
	pub virt: NonNull<u8>,
	/// The size in bytes
	pub size: usize,
	/// The PCI device this region belongs to.
	_pci: &'a PCI,
}

/// A specific PCI bus.
pub struct Bus<'a> {
	pci: &'a PCI,
	bus: u8,
}

impl<'a> Bus<'a> {
	pub fn iter(&self) -> IterBus<'a> {
		IterBus {
			pci: self.pci,
			bus: self.bus,
			device: 0,
		}
	}
}

impl<'a> From<Bus<'a>> for Option<Header<'a>> {
	fn from(f: Bus<'a>) -> Self {
		f.pci.get(f.bus, 0, 0)
	}
}

/// A specific PCI device.
pub struct Device<'a> {
	pub pci: &'a PCI,
	bus: u8,
	device: u8,
}

impl<'a> Device<'a> {
	pub fn vendor_id(&self) -> u16 {
		self.header().common().vendor_id.get().into()
	}

	pub fn device_id(&self) -> u16 {
		self.header().common().device_id.get().into()
	}

	pub fn header(&self) -> Header {
		self.pci.get(self.bus, self.device, 0).unwrap()
	}
}

impl<'a> From<Device<'a>> for Option<Header<'a>> {
	fn from(f: Device<'a>) -> Self {
		f.pci.get(f.bus, f.device, 0)
	}
}

/// A specific PCI function.
pub struct Function<'a> {
	pci: &'a PCI,
	bus: u8,
	device: u8,
	function: u8,
}

impl<'a> From<Function<'a>> for Option<Header<'a>> {
	fn from(f: Function<'a>) -> Self {
		f.pci.get(f.bus, f.device, f.function)
	}
}

pub struct IterPCI<'a> {
	pci: &'a PCI,
	bus: u8,
}

pub struct IterBus<'a> {
	pci: &'a PCI,
	bus: u8,
	device: u8,
}

pub struct IterDevice<'a> {
	pci: &'a PCI,
	bus: u8,
	device: u8,
	function: u8,
}

fn log(msg: &str) {
	let (a, l) = (msg.as_bytes() as *const [u8]).to_raw_parts();
	unsafe { kernel::sys_log(a.cast(), l) };
}

impl<'a> Iterator for IterPCI<'a> {
	type Item = Bus<'a>;

	fn next(&mut self) -> Option<Bus<'a>> {
		if self.bus == 0xff {
			return None;
		} else if self.bus == 0 {
			let h = self.pci.get_unchecked(0, 0, 0);
			if h.common().header_type.get() & 0x80 == 0 {
				self.bus = 0xff;
				return Some(Bus {
					pci: self.pci,
					bus: 0,
				});
			}
		}

		self.bus += 1;
		let h = self.pci.get_unchecked(0, 0, self.bus);
		if h.common().vendor_id.get() != 0xffff.into() {
			self.bus = 0xff;
			None
		} else {
			Some(Bus {
				pci: self.pci,
				bus: self.bus,
			})
		}
	}
}

impl<'a> Iterator for IterBus<'a> {
	type Item = Device<'a>;

	fn next(&mut self) -> Option<Device<'a>> {
		while self.device < 32 {
			let dev = self.device;
			self.device += 1;
			if self.pci.get(self.bus, dev, 0).is_some() {
				return Some(Device {
					pci: self.pci,
					bus: self.bus,
					device: dev,
				});
			}
		}
		None
	}
}

pub enum FunctionItem<'a> {
	Header(Header<'a>),
	Bus(Bus<'a>),
}

impl<'a> Iterator for IterDevice<'a> {
	type Item = FunctionItem<'a>;

	fn next(&mut self) -> Option<FunctionItem<'a>> {
		if self.function == 0xff {
			None
		} else {
			let h = self.pci.get_unchecked(self.bus, self.device, self.function);
			if h.common().vendor_id.get() == 0xffff.into() {
				self.function = 0xff;
				None
			} else {
				let ht = h.common().header_type.get();
				if ht & 0x80 > 0 {
					if let Header::H1(h) = h {
						if h.common.class_code.get() == 0x6 && h.common.subclass.get() == 0x4 {
							let sb = h.secondary_bus_number.get();
							Some(FunctionItem::Bus(Bus {
								pci: self.pci,
								bus: sb,
							}))
						} else {
							Some(FunctionItem::Header(Header::H1(h)))
						}
					} else {
						Some(FunctionItem::Header(h))
					}
				} else {
					Some(FunctionItem::Header(h))
				}
			}
		}
	}
}

fn log_u16(n: u16) {
	let mut buf = [0u8; 4];
	buf[0] = ((n >> 12) & 0xf) as u8;
	buf[1] = ((n >>  8) & 0xf) as u8;
	buf[2] = ((n >>  4) & 0xf) as u8;
	buf[3] = ((n >>  0) & 0xf) as u8;
	for c in buf.iter_mut() {
		*c += if *c < 10 { b'0' } else { b'a' - 10 };
	}
	log(core::str::from_utf8(&buf).unwrap());
}
