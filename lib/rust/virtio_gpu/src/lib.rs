#![no_std]

mod controlq;
mod cursorq;

pub use controlq::resource::create_2d::Format;
pub use controlq::Rect;

use core::convert::TryInto;
use core::fmt;
use core::mem;
use core::pin::Pin;
use core::ptr::NonNull;
use simple_endian::{u32le, u64le};
use vcell::VolatileCell;

#[allow(dead_code)]
const FEATURE_VIRGL: u32 = 0x1;
const FEATURE_EDID: u32 = 0x2;

#[repr(C)]
struct Config {
	events_read: VolatileCell<u32le>,
	events_clear: VolatileCell<u32le>,
	num_scanouts: VolatileCell<u32le>,
	_reserved: u32le,
}

impl Config {
	const EVENT_DISPLAY: u32 = 0x1;
}

#[repr(C)]
struct ControlHeader {
	ty: u32le,
	flags: u32le,
	fence_id: u64le,
	context_id: u32le,
	_padding: u32le,
}

impl ControlHeader {
	const CMD_GET_DISPLAY_INFO: u32 = 0x100;
	const CMD_RESOURCE_CREATE_2D: u32 = 0x101;
	const CMD_RESOURCE_UNREF: u32 = 0x102;
	const CMD_SET_SCANOUT: u32 = 0x103;
	const CMD_RESOURCE_FLUSH: u32 = 0x104;
	const CMD_TRANSFER_TO_HOST_2D: u32 = 0x105;
	const CMD_RESOURCE_ATTACH_BACKING: u32 = 0x106;
	const CMD_RESOURCE_DETACH_BACKING: u32 = 0x107;
	const CMD_GET_CAPSET_INFO: u32 = 0x108;
	const CMD_GET_CAPSET: u32 = 0x109;
	const CMD_GET_EDID: u32 = 0x110;

	const CMD_UPDATE_CURSOR: u32 = 0x300;
	const CMD_MOVE_CURSOR: u32 = 0x301;

	const RESP_OK_NODATA: u32 = 0x1100;
	const RESP_OK_DISPLAY_INFO: u32 = 0x1101;
	const RESP_OK_CAPSET_INFO: u32 = 0x1102;
	const RESP_OK_CAPSET: u32 = 0x1103;
	const RESP_OK_EDID: u32 = 0x1104;

	const RESP_ERR_UNSPEC: u32 = 0x1200;
	const RESP_ERR_OUT_OF_MEMORY: u32 = 0x1201;
	const RESP_ERR_INVALID_SCANOUT_ID: u32 = 0x1202;
	const RESP_ERR_INVALID_RESOURCE_ID: u32 = 0x1203;
	const RESP_ERR_INVALID_CONTEXT_ID: u32 = 0x1204;
	const RESP_ERR_INVALID_PARAMETER: u32 = 0x1205;

	const FLAG_FENCE: u32 = 0x1;

	fn new(ty: u32, fence: Option<u64>) -> Self {
		Self {
			ty: ty.into(),
			flags: fence.map(|_| ControlHeader::FLAG_FENCE).unwrap_or(0).into(),
			fence_id: fence.unwrap_or(0).into(),
			context_id: 0.into(),
			_padding: 0.into(),
		}
	}
}

impl fmt::Debug for ControlHeader {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let mut d = f.debug_struct(stringify!(ControlHeader));

		let ty = match self.ty.into() {
			Self::CMD_GET_DISPLAY_INFO => "CMD_GET_DISPLAY_INFO",
			Self::CMD_RESOURCE_CREATE_2D => "CMD_RESOURCE_CREATE_2D",
			Self::CMD_RESOURCE_UNREF => "CMD_RESOURCE_UNREF",
			Self::CMD_SET_SCANOUT => "CMD_SET_SCANOUT",
			Self::CMD_RESOURCE_FLUSH => "CMD_RESOURCE_FLUSH",
			Self::CMD_TRANSFER_TO_HOST_2D => "CMD_TRANSFER_TO_HOST_2D",
			Self::CMD_RESOURCE_ATTACH_BACKING => "CMD_RESOURCE_ATTACH_BACKING",
			Self::CMD_RESOURCE_DETACH_BACKING => "CMD_RESOURCE_DETACH_BACKING",
			Self::CMD_GET_CAPSET_INFO => "CMD_GET_CAPSET_INFO",
			Self::CMD_GET_CAPSET => "CMD_GET_CAPSET",
			Self::CMD_GET_EDID => "CMD_GET_EDID",

			Self::CMD_UPDATE_CURSOR => "CMD_UPDATE_CURSOR",
			Self::CMD_MOVE_CURSOR => "CMD_MOVE_CURSOR",

			Self::RESP_OK_NODATA => "RESP_OK_NODATA",
			Self::RESP_OK_DISPLAY_INFO => "RESP_OK_DISPLAY_INFO",
			Self::RESP_OK_CAPSET_INFO => "RESP_OK_CAPSET_INFO",
			Self::RESP_OK_CAPSET => "RESP_OK_CAPSET",
			Self::RESP_OK_EDID => "RESP_OK_EDID",

			Self::RESP_ERR_UNSPEC => "RESP_ERR_UNSPEC",
			Self::RESP_ERR_OUT_OF_MEMORY => "RESP_ERR_OUT_OF_MEMORY",
			Self::RESP_ERR_INVALID_SCANOUT_ID => "RESP_ERR_INVALID_SCANOUT_ID",
			Self::RESP_ERR_INVALID_RESOURCE_ID => "RESP_ERR_INVALID_RESOURCE_ID",
			Self::RESP_ERR_INVALID_CONTEXT_ID => "RESP_ERR_INVALID_CONTEXT_ID",
			Self::RESP_ERR_INVALID_PARAMETER => "RESP_ERR_INVALID_PARAMETER",

			_ => "",
		};
		if ty == "" {
			d.field("type", &format_args!("0x{:x}", self.ty));
		} else {
			d.field("type", &format_args!("{}", ty));
		}

		let flags = u32::from(self.flags);
		if flags == Self::FLAG_FENCE {
			d.field("flags", &format_args!("FLAG_FENCE"));
		} else if flags & Self::FLAG_FENCE > 0 {
			d.field(
				"flags",
				&format_args!("FLAG_FENCE | 0x{:x}", flags & !Self::FLAG_FENCE),
			);
		} else {
			d.field("flags", &format_args!("0x{:x}", flags));
		}

		d.field("fence_id", &u64::from(self.fence_id));
		d.field("context_id", &u32::from(self.context_id));
		d.finish()
	}
}

pub struct Device<'a> {
	config: &'a Config,
	notify: &'a virtio::pci::Notify,
	controlq: virtio::queue::Queue<'a>,
	cursorq: virtio::queue::Queue<'a>,
}

impl<'a> Device<'a> {
	/// Setup a GPU device
	///
	/// This is meant to be used as a handler by the `virtio` crate.
	pub fn new(
		common: &'a virtio::pci::CommonConfig,
		device: &'a virtio::pci::DeviceConfig,
		notify: &'a virtio::pci::Notify,
	) -> Result<Self, SetupError> {
		let features = FEATURE_EDID;
		common.device_feature_select.set(0.into());

		let features = u32le::from(features) & common.device_feature.get();
		common.device_feature.set(features);

		common.device_status.set(
			virtio::pci::CommonConfig::STATUS_ACKNOWLEDGE
				| virtio::pci::CommonConfig::STATUS_DRIVER
				| virtio::pci::CommonConfig::STATUS_FEATURES_OK,
		);
		// TODO check device status to ensure features were enabled correctly.

		let config = unsafe { device.cast::<Config>() };

		let controlq = virtio::queue::Queue::<'a>::new(common, 0, 8).expect("OOM");
		let cursorq = virtio::queue::Queue::<'a>::new(common, 1, 8).expect("OOM");

		common.device_status.set(
			virtio::pci::CommonConfig::STATUS_ACKNOWLEDGE
				| virtio::pci::CommonConfig::STATUS_DRIVER
				| virtio::pci::CommonConfig::STATUS_FEATURES_OK
				| virtio::pci::CommonConfig::STATUS_DRIVER_OK,
		);

		Ok(Self {
			config,
			controlq,
			cursorq,
			notify,
		})
	}

	pub unsafe fn init_scanout(
		&mut self,
		format: Format,
		rect: Rect,
		backend: NonNull<kernel::Page>,
		count: usize,
	) -> Result<(), InitScanoutError> {
		let res_id = 1;
		let scan_id = 0;

		// Get storage phys addresses
		assert!(count <= 16, "todo: support large page counts");
		let mut phys_addrs = [0; 16];
		let phys_addrs = &mut phys_addrs[..count];
		let ret = unsafe {
			kernel::mem_physical_address(
				backend.as_ptr(),
				phys_addrs.as_mut_ptr(),
				phys_addrs.len(),
			)
		};
		assert_eq!(ret.status, 0, "backend not allocated");
		//let phys_addrs = &phys_addrs[..ret.value];
		let phys_addrs = &phys_addrs[..];
		kernel::dbg!(backend);
		phys_addrs.iter().for_each(|a| {
			kernel::dbg!(format_args!("0x{:x}", a));
		});

		// Response buffer
		let mut resp_buffer = ControlHeader::new(0, None);
		let mut resp_buffer = Pin::new(&mut resp_buffer);
		let res_ptr = &mut *resp_buffer as *mut _ as usize;
		let (ppn, offt) = (res_ptr & !kernel::Page::MASK, res_ptr & kernel::Page::MASK);
		let mut phys = 0;
		let ret = unsafe { kernel::mem_physical_address(ppn as *const _, &mut phys, 1) };
		assert_eq!(ret.status, 0, "Failed DMA get phys address");
		let resp_data = (phys + offt, mem::size_of::<ControlHeader>(), true);

		// Create resource
		let res =
			controlq::resource::Create2D::new(res_id, format, rect.width(), rect.height(), Some(0));
		let res = Pin::new(&res);
		let res_ptr = &*res as *const _ as usize;
		let (ppn, offt) = (res_ptr & !kernel::Page::MASK, res_ptr & kernel::Page::MASK);
		let mut phys = 0;
		let ret = unsafe { kernel::mem_physical_address(ppn as *const _, &mut phys, 1) };
		assert_eq!(ret.status, 0, "Failed DMA get phys address");
		let data = [
			(
				phys + offt,
				mem::size_of::<controlq::resource::Create2D>(),
				false,
			),
			resp_data,
		];
		self.controlq
			.send(data.iter().copied())
			.expect("failed to send data");
		self.flush();
		self.controlq.wait_for_used(None);

		// Attach storage
		#[repr(C)]
		struct Storage {
			attach: controlq::resource::AttachBacking,
			mem_entries: [controlq::resource::MemoryEntry; 16],
		}
		let mut storage = Storage {
			attach: controlq::resource::AttachBacking::new(
				res_id,
				phys_addrs.len().try_into().unwrap(),
				Some(0),
			),
			mem_entries: [controlq::resource::MemoryEntry::new(0, 0); 16],
		};
		for (w, r) in storage
			.mem_entries
			.iter_mut()
			.zip(phys_addrs.iter().copied())
		{
			*w = controlq::resource::MemoryEntry::new(
				r.try_into().unwrap(),
				kernel::Page::SIZE.try_into().unwrap(),
			);
		}
		let sto_ptr = &storage as *const _ as usize;
		let (ppn, offt) = (sto_ptr & !kernel::Page::MASK, sto_ptr & kernel::Page::MASK);
		let mut phys = 0;
		let ret = unsafe { kernel::mem_physical_address(ppn as *const _, &mut phys, 1) };
		assert_eq!(ret.status, 0, "Failed DMA get phys address");
		let size = mem::size_of::<controlq::resource::AttachBacking>()
			+ mem::size_of::<controlq::resource::MemoryEntry>() * phys_addrs.len();
		let data = [(phys + offt, size, false), resp_data];
		self.controlq
			.send(data.iter().copied())
			.expect("failed to send data");
		self.flush();
		self.controlq.wait_for_used(None);

		// Attach scanout
		let scanout = controlq::SetScanout::new(scan_id, res_id, rect, Some(0));
		let sto_ptr = &scanout as *const _ as usize;
		let (ppn, offt) = (sto_ptr & !kernel::Page::MASK, sto_ptr & kernel::Page::MASK);
		let mut phys = 0;
		let ret = unsafe { kernel::mem_physical_address(ppn as *const _, &mut phys, 1) };
		assert_eq!(ret.status, 0, "Failed DMA get phys address");
		let data = [
			(phys + offt, mem::size_of::<controlq::SetScanout>(), false),
			resp_data,
		];
		self.controlq
			.send(data.iter().copied())
			.expect("failed to send data");
		self.flush();
		self.controlq.wait_for_used(None);

		Ok(())
	}

	pub fn draw(&mut self, rect: Rect) -> Result<(), DrawError> {
		let res_id = 1;

		// Response buffer
		let mut resp_buffer = ControlHeader::new(0, None);
		let mut resp_buffer = Pin::new(&mut resp_buffer);
		let res_ptr = &mut *resp_buffer as *mut _ as usize;
		let (ppn, offt) = (res_ptr & !kernel::Page::MASK, res_ptr & kernel::Page::MASK);
		let mut phys = 0;
		let ret = unsafe { kernel::mem_physical_address(ppn as *const _, &mut phys, 1) };
		assert_eq!(ret.status, 0, "Failed DMA get phys address");
		let resp_data = (phys + offt, mem::size_of::<ControlHeader>(), true);

		// Transfer to host
		let res = controlq::TransferToHost2D::new(res_id, 0, rect, Some(0));
		let res_ptr = &res as *const _ as usize;
		let (ppn, offt) = (res_ptr & !kernel::Page::MASK, res_ptr & kernel::Page::MASK);
		let mut phys = 0;
		let ret = unsafe { kernel::mem_physical_address(ppn as *const _, &mut phys, 1) };
		assert_eq!(ret.status, 0, "Failed DMA get phys address");
		let data = [
			(
				phys + offt,
				mem::size_of::<controlq::TransferToHost2D>(),
				false,
			),
			resp_data,
		];
		self.controlq
			.send(data.iter().copied())
			.expect("failed to send data");
		self.flush();
		self.controlq.wait_for_used(None);

		// Flush resource
		let flush = controlq::resource::Flush::new(res_id.try_into().unwrap(), rect, Some(0));
		let flush = Pin::new(&flush);
		let sto_ptr = &*flush as *const _ as usize;
		let (ppn, offt) = (sto_ptr & !kernel::Page::MASK, sto_ptr & kernel::Page::MASK);
		let mut phys = 0;
		let ret = unsafe { kernel::mem_physical_address(ppn as *const _, &mut phys, 1) };
		assert_eq!(ret.status, 0, "Failed DMA get phys address");
		let data = [
			(
				phys + offt,
				mem::size_of::<controlq::resource::Flush>(),
				false,
			),
			resp_data,
		];
		self.controlq
			.send(data.iter().copied())
			.expect("failed to send data");
		self.flush();
		self.controlq.wait_for_used(None);

		Ok(())
	}

	fn flush(&self) {
		self.notify.send(0)
	}
}

impl virtio::pci::Device for Device<'_> {}

#[derive(Debug)]
pub enum SetupError {}

#[derive(Debug)]
pub enum InitScanoutError {}

#[derive(Debug)]
pub enum DrawError {}
