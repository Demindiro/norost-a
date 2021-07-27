use crate::block;
use core::alloc::Allocator;
use core::fmt;
use core::num::NonZeroU8;
use core::ptr::NonNull;
use simple_endian::{u16le, u32le, u64le};
use vcell::VolatileCell;

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

/// Setup a virtio device on a PCI bus.
pub fn new_device<'a, A>(device: pci::Device<'a>) -> Result<Device<'a, A>, SetupError>
where
    A: Allocator,
{
    if device.vendor_id() != 0x1af4 {
        return Err(SetupError::UnknownVendor(device.vendor_id()));
    }

    if device.device_id() != 0x1001 {
        return Err(SetupError::UnknownDevice(device.device_id()));
    }

    let cmd = pci::HeaderCommon::COMMAND_MMIO_MASK | pci::HeaderCommon::COMMAND_BUS_MASTER_MASK;
    use core::fmt::Write;

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

    let mut mmio = [None, None, None, None, None, None];
    assert_eq!(mmio.len(), pci::Header0::BASE_ADDRESS_COUNT as usize);

    let mut setup_mmio = |bar: u8, offset: u32| -> NonNull<u8> {
        let bar_id = bar.into();
        writeln!(kernel::SysLog, "OFFT {:x}", offset);
        if let Option::<pci::MMIO>::Some(mmio) = &mut mmio[bar_id] {
            unsafe { NonNull::new_unchecked(mmio.virt.as_ptr().add(offset as usize)) }
        } else {
            if let Option::<NonZeroU8>::Some(bar) = bar_sizes[bar_id] {
                let bar = bar.get();
                let m = device
                    .pci
                    .allocate_mmio((bar & !BAR_64_FLAG).into(), u8::from(bar & BAR_64_FLAG > 0))
                    .expect("Failed to allocate MMIO space");
                writeln!(kernel::SysLog, "{:?}", m.virt);
                writeln!(kernel::SysLog, "{:x}", m.physical);
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

    block::Device::new(common_config, device_config, notify_config)
        .map(Device::Block)
        .map_err(SetupError::Block)
}

pub enum Device<'a, A>
where
    A: Allocator,
{
    Block(block::Device<'a, A>),
}

pub enum SetupError {
    /// The vendor ID is not that of a known virtio device.
    UnknownVendor(u16),
    /// The device ID is not that of a known virtio device.
    UnknownDevice(u16),
    /// The header is not of an expected type, i.e. `header_type != 0`.
    UnexpectedHeaderType(u8),
    /// An error occured while setting up a block device.
    Block(block::SetupError),
}

impl fmt::Debug for SetupError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::UnknownVendor(id) => write!(f, "Unknown vendor 0x{:x}", id),
            Self::UnknownDevice(id) => write!(f, "Unknown device 0x{:x}", id),
            Self::UnexpectedHeaderType(t) => write!(f, "Unexpected header type 0x{:x}", t),
            Self::Block(b) => <block::SetupError as fmt::Debug>::fmt(b, f),
        }
    }
}
