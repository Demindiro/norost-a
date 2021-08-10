use alloc::prelude::v1::*;
use core::alloc::Allocator;
use core::fmt;
use core::num::NonZeroU8;
use core::ptr::NonNull;
use simple_endian::{u16le, u32le, u64le};
use vcell::VolatileCell;

/// The type of a device handler
/*
pub type DeviceHandler<A> =
	for<'a> fn(
		&'a CommonConfig,
		&'a DeviceConfig,
		&'a Notify,
		A,
	) -> Result<Box<dyn Device<A> + 'a, A>, Box<dyn DeviceHandlerError<A> + 'a, A>>;
	*/

// Using a newtype because https://github.com/rust-lang/rust/issues/64552
pub struct DeviceHandler<A: Allocator>(
	pub  for<'a> fn(
		&'a CommonConfig,
		&'a DeviceConfig,
		&'a Notify,
		A,
	) -> Result<Box<dyn Device<A> + 'a, A>, Box<dyn DeviceHandlerError<A> + 'a, A>>,
);

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
pub struct ISRConfig {
	pub status: VolatileCell<u8>,
}

impl ISRConfig {
	pub const QUEUE_INTERRUPT: u8 = 0x1;
	pub const DEVICE_CONFIGURATION_INTERRUPT: u8 = 0x2;
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

trait Log2 {
	type Output;

	fn log2(&self) -> Self::Output;
}

impl Log2 for u32 {
	type Output = u8;

	#[track_caller]
	fn log2(&self) -> Self::Output {
		assert_ne!(*self, 0, "Can't take logarithm of 0");
		let mut l = 0;
		let mut s = *self;
		while s & 1 == 0 {
			l += 1;
			s >>= 1;
		}
		l
	}
}

pub trait DeviceHandlerError<A>
where
	Self: fmt::Debug,
	A: Allocator,
{
}

/// A list of handlers that can initialize devices.
pub trait DeviceHandlers<'a, A>
where
	A: Allocator + 'a,
{
	fn can_handle(&self, ty: DeviceType) -> bool;

	/// Get a handler for a device of the given type.
	fn handle(
		&self,
		device_type: DeviceType,
		common: &'a CommonConfig,
		device: &'a DeviceConfig,
		notify: &'a Notify,
		allocator: A,
	) -> Result<Box<dyn Device<A> + 'a, A>, Box<dyn DeviceHandlerError<A> + 'a, A>>;
}

/// Setup a virtio device on a PCI bus.
// TODO figure out how to get `A: Allocator` to work. I'm 99% certain there's a bug in the compiler
// because static lifetimes keep slipping in somehow
pub fn new_device<'a, A>(
	device: pci::Device<'a>,
	handlers: impl DeviceHandlers<'a, A>,
	allocator: A,
) -> Result<Box<dyn Device<A> + 'a, A>, SetupError<A>>
where
	A: Allocator + 'a,
{
	let key = DeviceType::new(device.vendor_id(), device.device_id());

	handlers
		.can_handle(key)
		.then(|| ())
		.ok_or(SetupError::NoHandler(key))?;

	let cmd = pci::HeaderCommon::COMMAND_MMIO_MASK | pci::HeaderCommon::COMMAND_BUS_MASTER_MASK;

	let header = match device.header() {
		pci::Header::H0(h) => h,
		h => return Err(SetupError::UnexpectedHeaderType(h.header_type())),
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
					Some(NonZeroU8::new(size.log2() | BAR_64_FLAG).unwrap())
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
		kernel::dbg!(cap.id());
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
		} else if cap.id() == 0x5 {
			// MSI
			kernel::dbg!("lesgo");
			#[repr(C)]
			struct MSI {
				msg_ctrl: VolatileCell<[u8; 2]>,
				msg_addr: VolatileCell<[u8; 8]>,
				_reserved: VolatileCell<[u8; 2]>,
				msg_data: VolatileCell<[u8; 2]>,
				mask: VolatileCell<[u8; 4]>,
				pending: VolatileCell<[u8; 4]>,
			}
			let data = unsafe { cap.data::<MSI>() };
			kernel::dbg!("set msi");
			data.msg_ctrl.set([0x1, 0x0]);
			kernel::dbg!("done msi");
		} else if cap.id() == 0x11 {
			// MSI-X
			kernel::dbg!("LESGO");
			#[repr(C)]
			#[repr(packed)]
			struct MSIX {
				msg_ctrl: VolatileCell<u16le>,
				table_offt_and_bir: VolatileCell<u32le>,
				pending_bit_offt_and_bir: VolatileCell<u32le>,
			}
			let data = unsafe { cap.data::<MSIX>() };
			kernel::dbg!("set msi-x");

			unsafe {
				data.msg_ctrl.set(((1 << 15) | 1).into());

				kernel::dbg!(data.msg_ctrl.get());
				kernel::dbg!(data.table_offt_and_bir.get());
				kernel::dbg!(data.pending_bit_offt_and_bir.get());
			}

			//data.table_offt_and_bir

			kernel::dbg!("done msi-x");

			/*
			match device.header() {
				::pci::Header::H0(h) => {
					kernel::dbg!("set msi-x h");
					kernel::dbg!("done msi-x h");
				}
				_ => panic!("bad header type"),
			}
			*/
		}
	}

	match device.header() {
		::pci::Header::H0(h) => {
			kernel::dbg!("set int");
			h.interrupt_line.set(0);
			h.interrupt_pin.set(1);
			kernel::dbg!("done int");
		}
		_ => panic!("bad header type"),
	}

	let mut mmio = [None, None, None, None, None, None];
	assert_eq!(mmio.len(), pci::Header0::BASE_ADDRESS_COUNT as usize);

	let mut setup_mmio = |bar: u8, offset: u32| -> NonNull<u8> {
		let bar_id = bar.into();
		if let Option::<pci::MMIO>::Some(mmio) = &mut mmio[bar_id] {
			unsafe { NonNull::new_unchecked(mmio.virt.as_ptr().add(offset as usize)) }
		} else {
			if let Option::<NonZeroU8>::Some(bar) = bar_sizes[bar_id] {
				let bar = bar.get();
				let m = device
					.pci
					.allocate_mmio((bar & !BAR_64_FLAG).into(), u8::from(bar & BAR_64_FLAG > 0))
					.expect("Failed to allocate MMIO space");
				if bar & BAR_64_FLAG > 0 {
					let mmio_phys = (m.physical as u32, (m.physical >> 32) as u32);
					header.set_base_address(bar_id, mmio_phys.0);
					header.set_base_address(bar_id + 1, mmio_phys.1);
				} else {
					header.set_base_address(bar_id, m.physical as u32);
				}
				header.set_command(cmd);
				let virt = unsafe { NonNull::new_unchecked(m.virt.as_ptr().add(offset as usize)) };
				mmio[bar_id] = Some(m);
				virt
			} else {
				unreachable!("I/O BARs aren't supported");
			}
		}
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
				.cast::<ISRConfig>()
				.as_ref()
		})
		.expect("No isr config map defined");

	kernel::dbg!("set isr");
	isr_config.status.set(0x0);
	kernel::dbg!("done isr");

	handlers
		.handle(key, common_config, device_config, notify_config, allocator)
		.map_err(SetupError::Handler)
}

/// # Safety
///
/// Because `TypeId` cannot be used on non-`'static` items, the `DeviceType` is used instead.
///
/// To ensure it is safe, there may be only exactly one device struct per `DeviceType`.
pub unsafe trait Device<A>
where
	A: Allocator,
{
	fn device_type(&self) -> DeviceType;
}

/// # Safety
///
/// Because `TypeId` cannot be used on non-`'static` items, the `DeviceType` is used instead.
///
/// To ensure it is safe, there may be only exactly one device struct per `DeviceType`.
pub unsafe trait StaticDeviceType<A>
where
	A: Allocator,
{
	fn device_type_of() -> DeviceType;
}

impl<'a, A> dyn Device<A> + 'a
where
	A: Allocator + 'a,
{
	pub fn is<D: StaticDeviceType<A> + Device<A>>(&self) -> bool {
		self.device_type() == D::device_type_of()
	}

	pub fn downcast_ref<'s, D: StaticDeviceType<A> + Device<A>>(&'s self) -> Option<&'s D> {
		if self.is::<D>() {
			unsafe { Some(&*(self as *const _ as *const D)) }
		} else {
			None
		}
	}

	pub fn downcast_mut<'s, D: StaticDeviceType<A> + Device<A>>(&'s mut self) -> Option<&mut D> {
		if self.is::<D>() {
			unsafe { Some(&mut *(self as *mut _ as *mut D)) }
		} else {
			None
		}
	}
}

pub enum SetupError<'a, A>
where
	A: Allocator + 'a,
{
	/// No handler was found for the device.
	NoHandler(DeviceType),
	/// The header is not of an expected type, i.e. `header_type != 0`.
	UnexpectedHeaderType(u8),
	/// An error occured in the device handler.
	Handler(Box<dyn DeviceHandlerError<A> + 'a, A>),
}

impl<'a, A> fmt::Debug for SetupError<'a, A>
where
	A: Allocator + 'a,
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::NoHandler(id) => write!(f, "no handler for {:?}", id),
			Self::UnexpectedHeaderType(t) => write!(f, "unexpected header type 0x{:x}", t),
			Self::Handler(e) => e.fmt(f),
		}
	}
}
