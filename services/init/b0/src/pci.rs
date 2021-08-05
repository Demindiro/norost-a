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
struct FuckingRust;

unsafe impl core::alloc::Allocator for FuckingRust {
	fn allocate(&self, layout: core::alloc::Layout) -> Result<core::ptr::NonNull<[u8]>, core::alloc::AllocError> {
		HEAP.allocate(layout)
	}

	unsafe fn deallocate(&self, ptr: core::ptr::NonNull<u8>, layout: core::alloc::Layout) {
		HEAP.deallocate(ptr, layout)
	}
}




// TODO move this to a dedicated process.
pub fn init_blk_device() {
	struct Handler;

	use virtio::pci::*;
	use alloc::prelude::v1::*;

	impl<'a, A> virtio::pci::DeviceHandlers<'a, A> for &Handler
	where
		A: core::alloc::Allocator + 'a,
	{
		fn can_handle(&self, ty: DeviceType) -> bool {
			ty.vendor() == 0x1af4 && ty.device() == 0x1001
		}

		fn handle(&self, ty: DeviceType,
			common: &'a CommonConfig,
			device: &'a DeviceConfig,
			notify: &'a Notify,
			allocator: A,
	   ) -> Result<Box<dyn Device<A> + 'a, A>, Box<dyn DeviceHandlerError<A> + 'a, A>> {
			assert!(ty.vendor() == 0x1af4 && ty.device() == 0x1001);
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
	for bus in pci.iter() {
		for dev in bus.iter() {
			if let Ok(dev) = virtio::pci::new_device(dev, &Handler, FuckingRust) {
				//dev.address
			}
		}
	}
}
