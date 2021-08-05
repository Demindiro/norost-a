#![no_std]

#![feature(allocator_api)]
#![feature(alloc_prelude)]

extern crate alloc;

mod sector;
#[cfg(feature = "fatfs_io")]
mod fatfs;

pub use sector::Sector;
#[cfg(feature = "fatfs_io")]
pub use crate::fatfs::Proxy;

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

/// A driver for a virtio block device.
pub struct BlockDevice<'a, A>
where
	A: Allocator,
{
	queue: queue::Queue<'a>,
	notify: &'a virtio::pci::Notify,
	/// The amount of sectors available
	capacity: u64,
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
	const READ: u32 = 0;
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

impl<'a, A> BlockDevice<'a, A>
where
	A: Allocator + 'a,
{
	/// Setup a block device
	///
	/// This is meant to be used as a handler by the `virtio` crate.
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
			capacity: blk_cfg.capacity.into(),
			_p: core::marker::PhantomData,
		}, allocator))
	}

	/// Write out sectors
	pub fn write<'s>(&'s mut self, data: impl AsRef<[Sector]> + 's, sector_start: u64) -> Result<(), WriteError> {
		let sector_count = data.as_ref().len();

		let header = RequestHeader {
			typ: RequestHeader::WRITE.into(),
			reserved: 0.into(),
			sector: sector_start.into(),
		};
		let status = RequestStatus { status: 111 };
		let (mut phys_header, mut phys_data, mut phys_status) = (0, 0, 0);
		let h = &header as *const _ as usize;
		let d = data.as_ref() as *const _ as *const u8 as usize;
		let s = &status as *const _ as usize;
		let (hp, ho) = (h & !0xfff, h & 0xfff);
		let (dp, d_) = (d & !0xfff, d & 0xfff);
		let (sp, so) = (s & !0xfff, s & 0xfff);
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
			(phys_data + d_, data.as_ref().len() * mem::size_of::<Sector>(), false),
			(phys_status + so, mem::size_of::<RequestStatus>(), true),
		];
		use core::fmt::Write;

		self.queue
			.send(data.iter().copied())
			.expect("Failed to send data");

		self.flush();

		self.queue.wait_for_used(None);

		Ok(())
	}

	/// Read in sectors
	pub fn read<'s>(&'s mut self, mut data: impl AsMut<[Sector]> + 's, sector_start: u64) -> Result<(), WriteError> {
		let sector_count = data.as_mut().len();

		let header = RequestHeader {
			typ: RequestHeader::READ.into(),
			reserved: 0.into(),
			sector: sector_start.into(),
		};
		let status = RequestStatus { status: 111 };
		let (mut phys_header, mut phys_data, mut phys_status) = (0, 0, 0);
		let h = &header as *const _ as usize;
		let d = data.as_mut() as *mut _ as *mut u8 as usize;
		let s = &status as *const _ as usize;
		let (hp, ho) = (h & !0xfff, h & 0xfff);
		let (dp, d_) = (d & !0xfff, d & 0xfff);
		let (sp, so) = (s & !0xfff, s & 0xfff);
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
			(phys_data + d_, data.as_mut().len() * mem::size_of::<Sector>(), true),
			(phys_status + so, mem::size_of::<RequestStatus>(), true),
		];

		self.queue
			.send(data.iter().copied())
			.expect("Failed to send data");

		self.flush();

		self.queue.wait_for_used(None);

		Ok(())
	}

	pub fn flush(&self) {
		self.notify.send(0);
	}
}

impl<A> Drop for BlockDevice<'_, A>
where
	A: Allocator
{
	fn drop(&mut self) {
		todo!("ensure the device doesn't read/write memory after being dropped");
	}
}

unsafe impl<'a, A> Device<A> for BlockDevice<'a, A>
where
	A: Allocator,
{
	fn device_type(&self) -> DeviceType {
		Self::device_type_of()
	}
}

unsafe impl<'a, A> StaticDeviceType<A> for BlockDevice<'a, A>
where
	A: Allocator,
{
	fn device_type_of() -> DeviceType {
		DeviceType::new(0x1af4, 0x1001)
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
}

impl fmt::Debug for WriteError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		/*
		f.write_str(match self {
		})
		*/
		Ok(())
	}
}
