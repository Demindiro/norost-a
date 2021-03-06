#![no_std]

mod sector;

pub use sector::Sector;

use core::convert::TryInto;
use core::fmt;
use core::mem;
use simple_endian::{u16le, u32le, u64le};
use virtio::pci::{CommonConfig, DeviceConfig, Notify};
use virtio::queue;

const SIZE_MAX: u32 = 1 << 1;
const SEG_MAX: u32 = 1 << 2;
const GEOMETRY: u32 = 1 << 4;
#[allow(dead_code)]
const RO: u32 = 1 << 5;
const BLK_SIZE: u32 = 1 << 6;
#[allow(dead_code)]
const FLUSH: u32 = 1 << 9;
const TOPOLOGY: u32 = 1 << 10;
#[allow(dead_code)]
const CONFIG_WCE: u32 = 1 << 11;
#[allow(dead_code)]
const DISCARD: u32 = 1 << 13;
#[allow(dead_code)]
const WRITE_ZEROES: u32 = 1 << 14;

#[allow(dead_code)]
const ANY_LAYOUT: u32 = 1 << 27;
#[allow(dead_code)]
const EVENT_IDX: u32 = 1 << 28;
#[allow(dead_code)]
const INDIRECT_DESC: u32 = 1 << 29;

/// A driver for a virtio block device.
pub struct BlockDevice<'a> {
	queue: queue::Queue<'a>,
	notify: virtio::pci::Notify<'a>,
	isr: &'a virtio::pci::ISR,
	/// The amount of sectors available
	_capacity: u64,
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
struct RequestStatus {
	status: u8,
}

use virtio::pci::*;

impl<'a> BlockDevice<'a> {
	/// Setup a block device
	///
	/// This is meant to be used as a handler by the `virtio` crate.
	pub fn new(
		common: &'a CommonConfig,
		device: &'a DeviceConfig,
		notify: Notify<'a>,
		isr: &'a virtio::pci::ISR,
	) -> Result<Self, SetupError> {
		let features = SIZE_MAX | SEG_MAX | GEOMETRY | BLK_SIZE | TOPOLOGY;
		common.device_feature_select.set(0.into());

		let features = u32le::from(features) & common.device_feature.get();
		common.device_feature.set(features);
		#[allow(dead_code)]
		const STATUS_DRIVER_OK: u8 = 0x4;

		common.device_status.set(
			CommonConfig::STATUS_ACKNOWLEDGE
				| CommonConfig::STATUS_DRIVER
				| CommonConfig::STATUS_FEATURES_OK,
		);
		// TODO check device status to ensure features were enabled correctly.

		let blk_cfg = unsafe { device.cast::<Config>() };

		// Set up queue.
		let queue = queue::Queue::<'a>::new(common, 0, 8, None).expect("OOM");

		common.device_status.set(
			CommonConfig::STATUS_ACKNOWLEDGE
				| CommonConfig::STATUS_DRIVER
				| CommonConfig::STATUS_FEATURES_OK
				| CommonConfig::STATUS_DRIVER_OK,
		);

		Ok(Self {
			queue,
			notify,
			isr,
			_capacity: blk_cfg.capacity.into(),
		})
	}

	/// Write out sectors
	pub fn write<'s>(
		&'s mut self,
		data: impl AsRef<[Sector]> + 's,
		sector_start: u64,
		wait: impl FnMut(),
	) -> Result<(), WriteError> {
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
			(
				(phys_header + ho).try_into().unwrap(),
				mem::size_of::<RequestHeader>().try_into().unwrap(),
				false,
			),
			(
				(phys_data + d_).try_into().unwrap(),
				(data.as_ref().len() * mem::size_of::<Sector>())
					.try_into()
					.unwrap(),
				false,
			),
			(
				(phys_status + so).try_into().unwrap(),
				mem::size_of::<RequestStatus>().try_into().unwrap(),
				true,
			),
		];

		self.queue
			.send(data.iter().copied(), None, None)
			.expect("Failed to send data");

		self.flush();

		self.queue.wait_for_used(None, wait);

		Ok(())
	}

	/// Read in sectors
	pub fn read<'s>(
		&'s mut self,
		mut data: impl AsMut<[Sector]> + 's,
		sector_start: u64,
		wait: impl FnMut(),
	) -> Result<(), WriteError> {
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
			(
				(phys_header + ho).try_into().unwrap(),
				mem::size_of::<RequestHeader>().try_into().unwrap(),
				false,
			),
			(
				(phys_data + d_).try_into().unwrap(),
				(data.as_mut().len() * mem::size_of::<Sector>())
					.try_into()
					.unwrap(),
				true,
			),
			(
				(phys_status + so).try_into().unwrap(),
				mem::size_of::<RequestStatus>().try_into().unwrap(),
				true,
			),
		];

		self.queue
			.send(data.iter().copied(), None, None)
			.expect("Failed to send data");

		self.flush();

		self.queue.wait_for_used(None, wait);

		Ok(())
	}

	pub fn flush(&self) {
		self.notify.send(0);
	}

	#[inline]
	pub fn was_interrupted(&self) -> bool {
		self.isr.read().queue_update()
	}
}

impl Drop for BlockDevice<'_> {
	fn drop(&mut self) {
		todo!("ensure the device doesn't read/write memory after being dropped");
	}
}

impl<'a> Device for BlockDevice<'a> {}

pub enum SetupError {}

impl fmt::Debug for SetupError {
	fn fmt(&self, _f: &mut fmt::Formatter) -> fmt::Result {
		//f.write_str(match self {
		//})
		Ok(())
	}
}

pub enum WriteError {}

impl fmt::Debug for WriteError {
	fn fmt(&self, _f: &mut fmt::Formatter) -> fmt::Result {
		/*
		f.write_str(match self {
		})
		*/
		Ok(())
	}
}
