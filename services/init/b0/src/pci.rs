// Thanks Rust, very cool ICE.
struct StackAlloc(core::cell::UnsafeCell<[u8; 4096]>, core::cell::Cell<usize>);

unsafe impl core::alloc::Allocator for StackAlloc {
	fn allocate(&self, layout: core::alloc::Layout) -> Result<core::ptr::NonNull<[u8]>, core::alloc::AllocError> {
		let s = self.1.get();
		let heap = unsafe { &mut *self.0.get() };
		let heap = &mut heap[s..];
		let offt = heap.as_ptr() as usize & (layout.align() - 1);
		let heap = &mut heap[offt..];
		let (ret, rem) = heap.split_at_mut(layout.size());
		self.1.set(s + offt + layout.size());
		Ok(core::ptr::NonNull::from(ret))
	}

	unsafe fn deallocate(&self, ptr: core::ptr::NonNull<u8>, layout: core::alloc::Layout) {
	}
}

unsafe impl Sync for StackAlloc {}

static HEAP: StackAlloc = unsafe { StackAlloc(core::cell::UnsafeCell::new([0; 4096]), core::cell::Cell::new(0)) };

// https://github.com/rust-lang/rust/issues/81270
#[derive(Clone, Copy)]
pub struct FuckingRust;

unsafe impl core::alloc::Allocator for FuckingRust {
	fn allocate(&self, layout: core::alloc::Layout) -> Result<core::ptr::NonNull<[u8]>, core::alloc::AllocError> {
		HEAP.allocate(layout)
	}

	unsafe fn deallocate(&self, ptr: core::ptr::NonNull<u8>, layout: core::alloc::Layout) {
		HEAP.deallocate(ptr, layout)
	}
}

pub static mut PCI: Option<pci::PCI> = None;

use alloc::prelude::v1::*;

pub static mut BLK: Option<Box<dyn virtio::pci::Device<FuckingRust>, FuckingRust>> = None;

// TODO move this to a dedicated process.
pub fn init_blk_device() {
	// We do a little trickery around the kernel ELF loader being shit (FIXME)
	HEAP.1.set(0);

	struct Handler;

	use alloc::prelude::v1::*;
	use virtio::pci::*;

	impl<'a, A> virtio::pci::DeviceHandlers<'a, A> for &Handler
	where
		A: core::alloc::Allocator + 'a,
	{
		fn can_handle(&self, ty: DeviceType) -> bool {
			virtio_block::BlockDevice::<A>::device_type_of() == ty
		}

		fn handle(&self, ty: DeviceType,
			common: &'a CommonConfig,
			device: &'a DeviceConfig,
			notify: &'a Notify,
			allocator: A,
	   ) -> Result<Box<dyn Device<A> + 'a, A>, Box<dyn DeviceHandlerError<A> + 'a, A>> {
			assert_eq!(virtio_block::BlockDevice::<A>::device_type_of(), ty);
			virtio_block::BlockDevice::new(common, device, notify, allocator)
		}
	}

	let mmio = pci::PhysicalMemory {
		physical: unsafe { super::device_tree::PCI_MMIO_PHYSICAL.0 },
		size: unsafe { super::device_tree::PCI_MMIO_PHYSICAL.1 },
		virt: super::device_tree::PCI_MMIO_ADDRESS.cast(),
	};
	let size = unsafe { super::device_tree::PCI_SIZE };
	let pci = unsafe { pci::PCI::new(super::device_tree::PCI_ADDRESS.cast(), size, &[mmio]) };
	unsafe {
		// A little more trickery
		core::ptr::write_volatile(&mut PCI as *mut _, None);
		core::ptr::write_volatile(&mut BLK as *mut _, None);
		PCI = Some(pci);
	}
	for bus in unsafe { PCI.as_mut().unwrap() }.iter() {
		for dev in bus.iter() {
			if let Ok(mut vdev) = virtio::pci::new_device(dev, &Handler, FuckingRust) {
				let dev = vdev.downcast_mut::<virtio_block::BlockDevice<FuckingRust>>().unwrap();
				#[repr(align(4096))]
				struct Aligned([u8; 512]);
				let mut data = Aligned([0; 512]);

				// Read
				dev.read(&mut data.0, 0);
				let num = u32::from_be_bytes([data.0[0], data.0[1], data.0[2], data.0[3]]);
				kernel::sys_log!("Read the number {}", num);

				// Write
				for (i, c) in (num + 1).to_be_bytes().iter().copied().enumerate() {
					data.0[i] = c;
				}
				assert_eq!(data.0.as_ptr() as usize & 0xfff, 0);
				dev.write(&data.0, 0).unwrap();
				unsafe {
					BLK = Some(vdev);
				}

				//core::mem::forget(vdev); // FIXME I suspect this is unsound. Investigate.
				return;
			}
		}
	}
}
