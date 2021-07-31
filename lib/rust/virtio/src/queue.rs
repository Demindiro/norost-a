//! Implementation of **split** virtqueues.

use core::convert::{TryFrom, TryInto};
use core::fmt;
use core::mem;
use core::ptr::NonNull;
use core::slice;
use core::sync::atomic::{self, Ordering};
use simple_endian::{u16le, u32le, u64le};

#[repr(C)]
#[repr(C)]
struct Descriptor {
	address: u64le,
	length: u32le,
	flags: u16le,
	next: u16le,
}

impl Descriptor {
	const NEXT: u16 = 0x1;
	const WRITE: u16 = 0x2;
	const AVAIL: u16 = 1 << 7;
	const USED: u16 = 1 << 15;
}

struct Avail;

#[repr(C)]
struct AvailHead {
	flags: u16le,
	index: u16le,
}

#[repr(C)]
struct AvailElement {
	index: u16le,
}

#[repr(C)]
/// Only for VIRTIO_F_EVENT_IDX
struct AvailTail {
	used_event: u16le,
}

struct Used;

#[repr(C)]
struct UsedHead {
	flags: u16le,
	index: u16le,
}

#[repr(C)]
struct UsedElement {
	index: u32le,
	length: u32le,
}

#[repr(C)]
struct UsedTail {
	avail_event: u16le,
}

pub struct Queue<'a> {
	config: &'a super::pci::CommonConfig,
	mask: u16,
	last_available: u16,
	last_used: u16,
	free_descriptors: [u16; 8],
	free_count: u16,
	descriptors: NonNull<Descriptor>,
	available: NonNull<Avail>,
	used: NonNull<Used>,
}

impl<'a> Queue<'a> {
	/// Create a new split virtqueue and attach it to the device.
	///
	/// The size must be a power of 2.
	pub fn new(
		config: &'a super::pci::CommonConfig,
		index: u16,
		max_size: u16,
	) -> Result<Self, OutOfMemory> {
		const DMA_ADDR: usize = 0x200_0000;

		// TODO ensure max_size is a power of 2
		let size = u16::from(config.queue_size.get()).min(max_size) as usize;
		let desc_size = mem::size_of::<Descriptor>() * size;
		let avail_size = mem::size_of::<AvailHead>()
			+ mem::size_of::<AvailElement>() * size
			+ mem::size_of::<AvailTail>();
		let used_size = mem::size_of::<UsedHead>()
			+ mem::size_of::<UsedElement>() * size
			+ mem::size_of::<UsedTail>();

		let align = |s| (s + 0xfff) & !0xfff;

		// TODO syscall to get virtual -> physical address map
		// How it will work: Pass in a virtual address, page count and buffer
		// The kernel will then write the corresponding PPNs to the buffer.
		// (Note that PPN != address! You need to shift it to the left for the actual address).
		// (Also note that the physical address may be larger than (1 << XLEN)).
		// TODO syscall should have an option to allocate a contiguous range of memory and an
		// option to allocate below 4G. Or maybe as a separate syscall.
		let ret = unsafe {
			kernel::dev_dma_alloc(
				DMA_ADDR as *mut kernel::Page,
				align(desc_size + avail_size) + align(used_size),
				0x2,
			)
		};
		let kernel::Return { status, value } = ret;
		assert_eq!(status, 0, "Failed DMA alloc");
		let mem = value as *mut u8;

		let descriptors = unsafe { NonNull::new_unchecked(mem.cast()) };
		let available = unsafe { NonNull::new_unchecked(mem.add(desc_size).cast()) };
		let used = unsafe {
			NonNull::<Used>::new_unchecked(mem.add(align(desc_size + avail_size)).cast())
		};

		let mut free_descriptors = [0; 8];
		for (i, u) in free_descriptors.iter_mut().enumerate() {
			*u = i as u16;
		}
		let free_count = 8;

		let mut phys = 0;
		let mem = unsafe { kernel::mem_physical_address(mem.cast(), &mut phys as *mut _, 1).value };
		assert_eq!(status, 0, "Failed DMA get phys address");

		let d_phys = phys;
		let a_phys = phys + desc_size;
		let u_phys = phys + align(desc_size + avail_size);

		config.queue_select.set(index.into());
		config.queue_descriptors.set((d_phys as u64).into());
		config.queue_driver.set((a_phys as u64).into());
		config.queue_device.set((u_phys as u64).into());
		config.queue_size.set((size as u16).into());
		config.queue_enable.set(1.into());

		use core::fmt::Write;
		writeln!(kernel::SysLog, "ger {:x} {:x} {:x}", d_phys, a_phys, u_phys);
		writeln!(kernel::SysLog, "aaaaaaa {:p}", used);

		Ok(Queue {
			config,
			mask: size as u16 - 1,
			last_available: 0,
			last_used: 0,
			free_descriptors,
			free_count,
			descriptors,
			available,
			used,
		})
	}

	/// Convert an iterator of `(address, data)` into a linked list of descriptors and put it in the
	/// available ring.
	pub fn send<I>(&mut self, iterator: I) -> Result<(), NoBuffers>
	where
		I: ExactSizeIterator<Item = (usize, usize, bool)>,
	{
		let count = iterator.len().try_into().unwrap();
		if count == 0 {
			// TODO is this really the right thing to do?
			return Ok(());
		}
		use core::fmt::Write;
		writeln!(kernel::SysLog, "aaaaaaa");
		unsafe {
			let size = usize::from(self.mask) + 1;
			let desc: &mut [Descriptor] =
				slice::from_raw_parts_mut(self.descriptors.as_ptr(), size);
			let avail_head = &mut *self.available.as_ptr().cast::<AvailHead>();
			let avail_ring = self
				.available
				.as_ptr()
				.cast::<u8>()
				.add(mem::size_of::<AvailHead>());
			let avail_ring: &mut [u16] = slice::from_raw_parts_mut(avail_ring.cast(), size);
			let used_head = &mut *self.used.as_ptr().cast::<UsedHead>();
			let used_ring = self
				.used
				.as_ptr()
				.cast::<u8>()
				.add(mem::size_of::<UsedHead>());
			let used_ring: &mut [UsedElement] = slice::from_raw_parts_mut(used_ring.cast(), size);

			if self.free_count < count {
				writeln!(kernel::SysLog, "ERR {} < {}", self.free_count, count);
				return Err(NoBuffers);
			}

			writeln!(kernel::SysLog, "====");
			let mut id = self.last_used;
			let mut head = u16le::from(0);
			let mut prev_next = &mut head;
			let mut iterator = iterator.peekable();
			while let Some((address, length, write)) = iterator.next() {
				self.free_count -= 1;
				let i = usize::from(self.free_descriptors[usize::from(self.free_count)]);
				desc[i].address =
					u64le::from(u64::try_from(address).expect("Address out of bounds"));
				desc[i].length = u32le::from(u32::try_from(length).expect("Length too large"));
				desc[i].flags = u16le::from(u16::from(write) * Descriptor::WRITE);
				desc[i].flags |=
					u16le::from(u16::from(iterator.peek().is_some()) * Descriptor::NEXT);
				*prev_next = u16le::from(i as u16);
				writeln!(
					kernel::SysLog,
					"length {} | flags 0b{:b} | next {}",
					desc[i].length,
					desc[i].flags,
					*prev_next
				);
				prev_next = &mut desc[i].next;
			}
			writeln!(kernel::SysLog, "FUUUUUUUUUUUU {}", head);

			avail_ring[usize::from(u16::from(avail_head.index) & self.mask)] = head.into();
			atomic::fence(Ordering::Release);
			writeln!(kernel::SysLog, "UUUU {}", avail_head.index);
			avail_head.index = u16::from(avail_head.index).wrapping_add(1).into();
			writeln!(kernel::SysLog, "____ {}", avail_head.index);
		}
		Ok(())
	}
}

pub struct OutOfMemory;

impl fmt::Debug for OutOfMemory {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "No free DMA memory")
	}
}

pub struct NoBuffers;

impl fmt::Debug for NoBuffers {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "No free buffers")
	}
}
