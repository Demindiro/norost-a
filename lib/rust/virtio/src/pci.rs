use core::convert::TryFrom;
use core::fmt;
use core::num::NonZeroU8;
use core::ptr::NonNull;
use simple_endian::{u16le, u32le, u64le};
use vcell::VolatileCell;

/// The type of a device handler

/*
// TODO move this to a separate "sync" crate.
struct Mutex<T> {
lock: core::sync::atomic::AtomicU8,
value: UnsafeCell<T>,
}

impl<T> Mutex<T> {
const fn new(value: T) -> Self {
Self {
lock: core::sync::atomic::AtomicU8::new(0),
value: UnsafeCell::new(value),
}
}

fn lock(&self) -> Guard<T> {
use core::sync::atomic::*;
while self.lock.compare_exchange_weak(0, 1, Ordering::Acquire, Ordering::Relaxed).is_err() {
}
Guard { lock: self }
}
}

unsafe impl<T> Sync for Mutex<T> {}

struct Guard<'a, T> {
lock: &'a Mutex<T>
}

impl<T> core::ops::Deref for Guard<'_, T> {
type Target = T;

fn deref(&self) -> &Self::Target {
unsafe { &*self.lock.value.get() }
}
}

impl<T> core::ops::DerefMut for Guard<'_, T> {
fn deref_mut(&mut self) -> &mut Self::Target {
unsafe { &mut *self.lock.value.get() }
}
}

impl<T> Drop for Guard<'_, T> {
fn drop(&mut self) {
use core::sync::atomic::*;
let _ret = self.lock.lock.compare_exchange(1, 0, Ordering::Release, Ordering::Relaxed);
debug_assert!(_ret.is_ok(), "failed to release lock");
}
}

/// All registered device handlers.
static DEVICE_HANDLERS: Mutex<BTreeMap<DeviceType, DeviceHandler>> = Mutex::new(BTreeMap::new());
*/

/// An identifier for a device type
#[derive(Clone, Copy, Hash, PartialOrd, Ord, Eq, PartialEq)]
pub struct DeviceType(u32);

impl DeviceType {
	/// Create a new device type identifier.
	#[inline(always)]
	pub fn new(vendor: u16, device: u16) -> Self {
		Self((u32::from(vendor) << 16) | u32::from(device))
	}

	/// Get the vendor of this device.
	#[inline(always)]
	pub fn vendor(&self) -> u16 {
		(self.0 >> 16) as u16
	}

	/// Get the type of device.
	#[inline(always)]
	pub fn device(&self) -> u16 {
		(self.0 & 0xffff) as u16
	}
}

impl fmt::Debug for DeviceType {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct(stringify!(DeviceType))
			.field("vendor", &self.vendor())
			.field("device", &self.device())
			.finish()
	}
}

#[repr(C)]
#[repr(packed)]
struct Capability {
	capability_length: VolatileCell<u8>,
	config_type: VolatileCell<u8>,
	base_address: VolatileCell<u8>,
	padding: [u8; 3],
	offset: VolatileCell<u32le>,
	length: VolatileCell<u32le>,
	more_stuff: VolatileCell<u32le>, // TODO
}

impl Capability {
	pub const COMMON_CONFIGURATION: u8 = 1;
	pub const NOTIFY_CONFIGURATION: u8 = 2;
	pub const ISR_CONFIGURATION: u8 = 3;
	pub const DEVICE_CONFIGURATION: u8 = 4;
	pub const PCI_CONFIGURATION: u8 = 5;
}

#[repr(C)]
pub struct CommonConfig {
	pub device_feature_select: VolatileCell<u32le>,
	pub device_feature: VolatileCell<u32le>,
	pub driver_feature_select: VolatileCell<u32le>,
	pub driver_feature: VolatileCell<u32le>,

	pub msix_config: VolatileCell<u16le>,
	pub queue_count: VolatileCell<u16le>,

	pub device_status: VolatileCell<u8>,
	pub config_generation: VolatileCell<u8>,

	pub queue_select: VolatileCell<u16le>,
	pub queue_size: VolatileCell<u16le>,
	pub queue_msix_vector: VolatileCell<u16le>,
	pub queue_enable: VolatileCell<u16le>,
	pub queue_notify_off: VolatileCell<u16le>,
	pub queue_descriptors: VolatileCell<u64le>,
	pub queue_driver: VolatileCell<u64le>,
	pub queue_device: VolatileCell<u64le>,
}

impl CommonConfig {
	pub const STATUS_RESET: u8 = 0x0;
	pub const STATUS_ACKNOWLEDGE: u8 = 0x1;
	pub const STATUS_DRIVER: u8 = 0x2;
	pub const STATUS_DRIVER_OK: u8 = 0x4;
	pub const STATUS_FEATURES_OK: u8 = 0x8;
	pub const STATUS_DEVICE_NEED_RESET: u8 = 0x40;
	pub const STATUS_FAILED: u8 = 0x80;
}

#[repr(C)]
pub struct ISR {
	status: VolatileCell<ISRStatus>,
}

impl ISR {
	/// Read the ISR status, clearing it.
	pub fn read(&self) -> ISRStatus {
		self.status.get()
	}
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct ISRStatus(u8);

impl ISRStatus {
	const QUEUE_INTERRUPT: u8 = 0x1;
	const CONFIGURATION_INTERRUPT: u8 = 0x2;

	/// Whether an interrupt for a queue update was issued.
	#[inline]
	pub fn queue_update(&self) -> bool {
		self.0 & Self::QUEUE_INTERRUPT > 0
	}

	/// Whether an interrupt for a configuration update was issued.
	#[inline]
	pub fn configuration_update(&self) -> bool {
		self.0 & Self::CONFIGURATION_INTERRUPT > 0
	}
}

/// Device specific configuration struct.
///
/// The fields of this struct are empty as there are no common fields.
#[repr(C)]
pub struct DeviceConfig(());

impl DeviceConfig {
	pub unsafe fn cast<'a, T>(&'a self) -> &'a T {
		&*(self as *const _ as *const _)
	}
}

#[repr(transparent)]
pub struct Notify(VolatileCell<u16le>);

impl Notify {
	pub fn send(&self, value: u16) {
		self.0.set(value.into());
	}
}

/// Setup a new virtio device on a PCI bus.
pub fn new_device<'a, D, H, R>(
	header: pci::Header<'a>,
	base_address_regions: &[Option<NonNull<()>>],
	handler: H,
) -> Result<D, R>
where
	D: Device + 'a,
	H: FnOnce(&'a CommonConfig, &'a DeviceConfig, &'a Notify, &'a ISR) -> Result<D, R>,
{
	let cmd = pci::HeaderCommon::COMMAND_MMIO_MASK | pci::HeaderCommon::COMMAND_BUS_MASTER_MASK;

	let header = match header {
		pci::Header::H0(h) => h,
		// TODO not actually unreachable, but meh.
		_ => unreachable!(),
	};

	const BAR_64_FLAG: u8 = 0x80;

	let mut bar_sizes = [None; pci::Header0::BASE_ADDRESS_COUNT as usize];
	let mut skip = false;
	for (i, bs) in bar_sizes.iter_mut().enumerate() {
		if skip {
			skip = false;
			continue;
		}
		let bar = header.base_address(i);
		if bar & pci::BAR_IO_SPACE > 0 {
			// Ignore I/O BARs for now.
		} else {
			*bs = match bar & pci::BAR_TYPE_MASK {
				0x0 => {
					header.set_base_address(i, 0xffff_ffff);
					let size = !(header.base_address(i) & !0xf) + 1;
					header.set_base_address(i, bar);
					(size > 0).then(|| NonZeroU8::new(size.log2() as u8).unwrap())
				}
				0x2 => panic!("Type bit 0x1 is reserved"),
				0x4 => {
					header.set_base_address(i, 0xffff_ffff);
					let size = !(header.base_address(i) & !0xf) + 1;
					header.set_base_address(i, bar);
					if size == 0x10 {
						// Technically possible. I doubt it'll happen in practice any time soon
						// though, so I can't be bothered.
						todo!("MMIO area larger than 4GB");
					}
					Some(NonZeroU8::new(size.log2() as u8 | BAR_64_FLAG).unwrap())
				}
				0x6 => panic!("Type bit 0x3 is reserved"),
				_ => unreachable!(),
			};
		}
	}

	let mut common_config = None;
	let mut notify_config = None;
	let mut isr_config = None;
	let mut device_config = None;
	let mut pci_config = None;

	for cap in header.capabilities() {
		if cap.id() == 0x9 {
			let cap = unsafe { cap.data::<Capability>() };
			if bar_sizes[usize::from(cap.base_address.get())].is_some() {
				match cap.config_type.get() {
					Capability::COMMON_CONFIGURATION => {
						if common_config.is_none() {
							common_config = Some(cap);
						}
					}
					Capability::NOTIFY_CONFIGURATION => {
						if notify_config.is_none() {
							notify_config = Some(cap);
						}
					}
					Capability::ISR_CONFIGURATION => {
						if isr_config.is_none() {
							isr_config = Some(cap);
						}
					}
					Capability::DEVICE_CONFIGURATION => {
						if device_config.is_none() {
							device_config = Some(cap);
						}
					}
					Capability::PCI_CONFIGURATION => {
						if pci_config.is_none() {
							pci_config = Some(cap);
						}
					}
					// There may exist other config types. We should ignore any we don't know.
					_ => (),
				}
			}
		}
	}

	let mmio = base_address_regions;
	assert_eq!(mmio.len(), pci::Header0::BASE_ADDRESS_COUNT as usize);

	let mut setup_mmio = |bar: u8, offset: u32| -> NonNull<u8> {
		let mmio = mmio[usize::from(bar)]
			.expect("BAR not mapped to region")
			.cast::<u8>();
		unsafe { NonNull::new_unchecked(mmio.as_ptr().add(offset as usize)) }
	};

	let common_config = common_config
		.map(|cfg| unsafe {
			setup_mmio(cfg.base_address.get(), cfg.offset.get().into())
				.cast::<CommonConfig>()
				.as_ref()
		})
		.expect("No common config map defined");

	let device_config = device_config
		.map(|cfg| unsafe {
			setup_mmio(cfg.base_address.get(), cfg.offset.get().into())
				.cast::<DeviceConfig>()
				.as_ref()
		})
		.expect("No common config map defined");

	let notify_config = notify_config
		.map(|cfg| unsafe {
			setup_mmio(cfg.base_address.get(), cfg.offset.get().into())
				.cast::<Notify>()
				.as_ref()
		})
		.expect("No common config map defined");

	let isr_config = isr_config
		.map(|cfg| unsafe {
			setup_mmio(cfg.base_address.get(), cfg.offset.get().into())
				.cast::<ISR>()
				.as_ref()
		})
		.expect("No isr config map defined");

	handler(common_config, device_config, notify_config, isr_config)
}

pub trait Device {}
