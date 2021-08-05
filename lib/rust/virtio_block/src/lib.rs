#![no_std]

#![feature(allocator_api)]
#![feature(alloc_prelude)]

extern crate alloc;

use alloc::prelude::v1::*;

use virtio::pci::{CommonConfig, DeviceConfig, Notify};
use virtio::queue;
use core::alloc::Allocator;
use core::fmt;
use core::mem;
use simple_endian::{u16le, u32le, u64le};
use vcell::VolatileCell;

const SIZE_MAX: u32 = 1 << 1;
const SEG_MAX: u32 = 1 << 2;
const GEOMETRY: u32 = 1 << 4;
const RO: u32 = 1 << 5;
const BLK_SIZE: u32 = 1 << 6;
const FLUSH: u32 = 1 << 9;
const TOPOLOGY: u32 = 1 << 10;
const CONFIG_WCE: u32 = 1 << 11;
const DISCARD: u32 = 1 << 13;
const WRITE_ZEROES: u32 = 1 << 14;

const ANY_LAYOUT: u32 = 1 << 27;
const EVENT_IDX: u32 = 1 << 28;
const INDIRECT_DESC: u32 = 1 << 29;

pub struct BlockDevice<'a, A>
where
	A: Allocator,
{
	queue: queue::Queue<'a>,
	notify: &'a virtio::pci::Notify,
	_p: core::marker::PhantomData<A>,
}

#[repr(C)]
struct Geometry {
	cylinders: u16,
	heads: u8,
	sectors: u8,
}

#[repr(C)]
struct Topology {
	physical_block_exp: u8,
	alignment_offset: u8,
	min_io_size: u16le,
	opt_io_size: u32le,
}

#[repr(C)]
struct Config {
	capacity: u64le,
	size_max: u32le,
	seg_max: u32le,
	geometry: Geometry,
	blk_size: u32le,
	topology: Topology,
	writeback: u8,
	_unused_0: [u8; 3],
	max_discard_sectors: u32le,
	max_discard_seg: u32le,
	discard_sector_alignment: u32le,
	max_write_zeroes_sectors: u32le,
	max_write_zeroes_seg: u32le,
	write_zeroes_may_unmap: u8,
	_unused_1: [u8; 3],
}

#[repr(C)]
struct RequestHeader {
	typ: u32le,
	reserved: u32le,
	sector: u64le,
}

impl RequestHeader {
	const WRITE: u32 = 1;
}

#[repr(C)]
struct RequestData {
	data: [u8; 512],
}

#[repr(C)]
struct RequestStatus {
	status: u8,
}

use virtio::pci::*;

/// Setup a block device
impl<'a, A> BlockDevice<'a, A>
where
	A: Allocator + 'a,
{
	pub fn new(
		common: &'a CommonConfig,
		device: &'a DeviceConfig,
		notify: &'a Notify,
		allocator: A,
	) -> Result<Box<dyn Device<A> + 'a, A>, Box<dyn DeviceHandlerError<A> + 'a, A>> {
		use core::fmt::Write;

		let features = SIZE_MAX | SEG_MAX | GEOMETRY | BLK_SIZE | TOPOLOGY;
		common.device_feature_select.set(0.into());

		let features = u32le::from(features) & common.device_feature.get();
		common.device_feature.set(features);
		const STATUS_DRIVER_OK: u8 = 0x4;

		common.device_status.set(
			CommonConfig::STATUS_ACKNOWLEDGE
			| CommonConfig::STATUS_DRIVER
			| CommonConfig::STATUS_FEATURES_OK,
			);
		// TODO check device status to ensure features were enabled correctly.

		let blk_cfg = unsafe { device.cast::<Config>() };

		// Set up queue.
		let queue = queue::Queue::<'a>::new(common, 0, 8).expect("OOM");

		common.device_status.set(
			CommonConfig::STATUS_ACKNOWLEDGE
			| CommonConfig::STATUS_DRIVER
			| CommonConfig::STATUS_FEATURES_OK
			| CommonConfig::STATUS_DRIVER_OK,
			);

		Ok(Box::new_in(Self {
			queue,
			notify,
			_p: core::marker::PhantomData,
		}, allocator))
	}

	/// Write out sectors
	pub fn write(&mut self, data: &[u8], sector_start: u64) -> Result<(), WriteError> {
		let sector_count = data.len() / 512;
		if sector_count * 512 != data.len() {
			return Err(WriteError::NotSectorSized);
		}

		writeln!(kernel::SysLog, "{:?}", data as *const _);
		writeln!(kernel::SysLog, "\"\"\"");
		for d in data {
			write!(kernel::SysLog, "{}", *d as char);
		}
		writeln!(kernel::SysLog, "\n");
		writeln!(kernel::SysLog, "\"\"\"");

		let header = RequestHeader {
			typ: RequestHeader::WRITE.into(),
			reserved: 0.into(),
			sector: sector_start.into(),
		};
		let status = RequestStatus { status: 111 };
		let (mut phys_header, mut phys_data, mut phys_status) = (0, 0, 0);
		let h = &header as *const _ as usize;
		let d = data as *const _ as *const u8 as usize;
		let s = &status as *const _ as usize;
		writeln!(kernel::SysLog, "WHAT {:x}", d);
		let (hp, ho) = (h & !0xfff, h & 0xfff);
		let (dp, d_) = (d & !0xfff, d & 0xfff);
		let (sp, so) = (s & !0xfff, s & 0xfff);
		writeln!(kernel::SysLog, "{:?}", (hp, ho));
		writeln!(kernel::SysLog, "{:?}", (dp, d_));
		writeln!(kernel::SysLog, "{:?}", (sp, so));
		let ret =
			unsafe { kernel::mem_physical_address(hp as *const _, &mut phys_header as *mut _, 1) };
		assert_eq!(ret.status, 0, "Failed DMA get phys address");
		let ret =
			unsafe { kernel::mem_physical_address(dp as *const _, &mut phys_data as *mut _, 1) };
		assert_eq!(ret.status, 0, "Failed DMA get phys address");
		let ret =
			unsafe { kernel::mem_physical_address(sp as *const _, &mut phys_status as *mut _, 1) };
		assert_eq!(ret.status, 0, "Failed DMA get phys address");

		let data = [
			(phys_header + ho, mem::size_of::<RequestHeader>(), false),
			(phys_data + d_, data.len(), false),
			(phys_status + so, mem::size_of::<RequestStatus>(), true),
		];
		use core::fmt::Write;
		writeln!(kernel::SysLog, "{:#?}", &data);

		self.queue
			.send(data.iter().copied())
			.expect("Failed to send data");
		writeln!(kernel::SysLog, "HONK");

		self.flush();
		loop {}

		Ok(())
	}

	pub fn flush(&self) {
		self.notify.send(0);
	}
}

impl<'a, A> Device<A> for BlockDevice<'a, A>
where
	A: Allocator,// + 'static,
{
	fn type_id(&self) -> core::any::TypeId {
		todo!()
		//core::any::TypeId::of::<BlockDevice<'static, A>>()
	}
}

pub enum SetupError {}

impl fmt::Debug for SetupError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.write_str(match self {
			_ => unreachable!(),
		})
	}
}

pub enum WriteError {
	NotSectorSized,
}

impl fmt::Debug for WriteError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.write_str(match self {
			Self::NotSectorSized => "The data's length is not a multiple of 512",
		})
	}
}
