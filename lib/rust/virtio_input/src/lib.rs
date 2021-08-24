//! # Virtio input library
//!
//! ## References
//!
//! https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-3390008

#![no_std]

mod ev;

use core::convert::{TryFrom, TryInto};
use core::fmt;
use core::mem;
use core::pin::Pin;
use core::ptr::NonNull;
use simple_endian::{i32le, u16le, u32le, u64le};
use vcell::VolatileCell;

#[allow(dead_code)]
const FEATURE_VIRGL: u32 = 0x1;
const FEATURE_EDID: u32 = 0x2;

#[repr(C)]
struct Config {
	select: VolatileCell<u8>,
	sub_select: VolatileCell<u8>,
	size: VolatileCell<u8>,
	_reserved: [u8; 5],
	u: ConfigUnion,
}

impl Config {
	const UNSET: u8 = 0x00;
	const ID_NAME: u8 = 0x01;
	const ID_SERIAL: u8 = 0x02;
	const ID_DEVIDS: u8 = 0x03;

	const PROP_BITS: u8 = 0x10;
	const EV_BITS: u8 = 0x11;
	const ABS_INFO: u8 = 0x12;
}

union ConfigUnion {
	string: mem::ManuallyDrop<VolatileCell<[u8; 128]>>,
	bitmap: mem::ManuallyDrop<VolatileCell<[u8; 128]>>,
	abs: mem::ManuallyDrop<AbsInfo>,
	ids: mem::ManuallyDrop<DevIds>,
}

#[repr(C)]
struct AbsInfo {
	min: VolatileCell<u32le>,
	max: VolatileCell<u32le>,
	fuzz: VolatileCell<u32le>,
	flat: VolatileCell<u32le>,
	res: VolatileCell<u32le>,
}

#[repr(C)]
struct DevIds {
	bustype: VolatileCell<u16le>,
	vendor: VolatileCell<u16le>,
	product: VolatileCell<u16le>,
	version: VolatileCell<u16le>,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct InputEvent {
	ty: u16le,
	code: u16le,
	value: i32le,
}

impl InputEvent {
	pub fn ty(&self) -> u16 {
		self.ty.into()
	}

	pub fn code(&self) -> u16 {
		self.code.into()
	}

	pub fn value(&self) -> i32 {
		self.value.into()
	}
}

impl fmt::Debug for InputEvent {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct(stringify!(InputEvent))
			.field("type", &self.ty())
			.field("code", &self.code())
			.field("value", &self.value())
			.finish()
	}
}

pub struct Device<'a> {
	config: &'a Config,
	notify: virtio::pci::Notify<'a>,
	eventq: virtio::queue::Queue<'a>,
	statusq: virtio::queue::Queue<'a>,
	events: NonNull<InputEvent>,
	events_phys_addr: usize,
}

impl<'a> Device<'a> {
	const MAX_EVENTS: u16 = 8;
	const MAX_STATUS: u16 = 8;

	/// Setup an input device
	///
	/// This is meant to be used as a handler by the `virtio` crate.
	pub fn new(
		common: &'a virtio::pci::CommonConfig,
		device: &'a virtio::pci::DeviceConfig,
		notify: virtio::pci::Notify<'a>,
		isr: &'a virtio::pci::ISR,
	) -> Result<Self, SetupError> {
		let features = 0;
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

		let eventq =
			virtio::queue::Queue::<'a>::new(common, 0, Self::MAX_EVENTS, None).expect("OOM");
		let statusq =
			virtio::queue::Queue::<'a>::new(common, 1, Self::MAX_STATUS, None).expect("OOM");

		// Push events to the event queue for the device to use.
		let events = dux::mem::allocate_range(None, 1, dux::RWX::RW).unwrap();
		let events = events.as_non_null_ptr().cast::<InputEvent>();

		let ev_ptr = events.as_ptr() as usize;
		let (ppn, offt) = (ev_ptr & !kernel::Page::MASK, ev_ptr & kernel::Page::MASK);
		let mut events_phys_addr = 0;
		let ret =
			unsafe { kernel::mem_physical_address(ppn as *const _, &mut events_phys_addr, 1) };

		let mut slf = Self {
			config,
			eventq,
			statusq,
			notify,
			events,
			events_phys_addr,
		};

		common.device_status.set(
			virtio::pci::CommonConfig::STATUS_ACKNOWLEDGE
				| virtio::pci::CommonConfig::STATUS_DRIVER
				| virtio::pci::CommonConfig::STATUS_FEATURES_OK
				| virtio::pci::CommonConfig::STATUS_DRIVER_OK,
		);

		assert_eq!(ret.status, 0, "Failed DMA get phys address");
		for i in 0..Self::MAX_EVENTS.into() {
			let size = mem::size_of::<InputEvent>();
			let phys = slf.events_phys_addr + offt + i * size;
			let data = [(phys.try_into().unwrap(), size.try_into().unwrap(), true)];
			slf.eventq
				.send(data.iter().copied(), None, None)
				.expect("failed to send to eventq");
		}
		slf.flush();

		Ok(slf)
	}

	/// Collect received entries.
	///
	/// This should be called periodically or on interrupt to prevent the queue from getting backed
	/// up.
	pub fn receive(&mut self, callback: &mut dyn FnMut(InputEvent)) -> Result<(), ReceiveError> {
		let evt = self.events;
		let evt_phys = self.events_phys_addr;
		let mut used = [(0, 0, false); Self::MAX_EVENTS as usize];
		let mut used_count = 0;
		self.eventq.collect_used(Some(&mut |_, phys, size| {
			let phys_u = usize::try_from(phys).expect("device returned bad physical address");
			let i = (phys_u - evt_phys) / mem::size_of::<InputEvent>();
			assert!(
				i < usize::from(Self::MAX_EVENTS),
				"device returned bad physical address"
			);

			callback(unsafe { *evt.as_ptr().add(i) });

			used[used_count] = (phys, size, true);
			used_count += 1;
		}));

		for u in used[..used_count].iter().copied() {
			self.eventq
				.send([u].iter().copied(), None, None)
				.expect("failed to send to eventq");
		}
		self.flush();

		Ok(())
	}

	pub fn name(&self, buf: &mut [u8; 128]) -> u8 {
		self.config.select.set(Config::ID_NAME);
		self.config.sub_select.set(0);
		let size = self.config.size.get().saturating_sub(1);
		buf.copy_from_slice(unsafe { &self.config.u.string.get() });
		size
	}

	pub fn serial_id(&self, buf: &mut [u8; 128]) -> u8 {
		self.config.select.set(Config::ID_SERIAL);
		self.config.sub_select.set(0);
		let size = self.config.size.get();
		buf.copy_from_slice(unsafe { &self.config.u.string.get() });
		size
	}

	pub fn ev_bits(&self, buf: &mut [u8; 128], ev: u8) -> u8 {
		self.config.select.set(Config::EV_BITS);
		self.config.sub_select.set(ev);
		let size = self.config.size.get();
		buf.copy_from_slice(unsafe { &self.config.u.bitmap.get() });
		size
	}

	fn flush(&self) {
		self.notify.send(0)
	}
}

impl virtio::pci::Device for Device<'_> {}

#[derive(Debug)]
pub enum SetupError {}

#[derive(Debug)]
pub enum ReceiveError {}
