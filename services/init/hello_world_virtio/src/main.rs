#![no_std]
#![no_main]
#![feature(allocator_api)] 
#![feature(global_asm)]
#![feature(panic_info_message)]
#![feature(ptr_metadata)]

use core::fmt::Write;
use core::ptr::NonNull;

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
	let _ = writeln!(kernel::SysLog, "Panic!");
	if let Some(m) = info.message() {
		let _ = writeln!(kernel::SysLog, "  Message: {}", m);
	}
	if let Some(l) = info.location() {
		let _ = writeln!(kernel::SysLog, "  Location: {}", l);
	}
	loop {}
}

global_asm!("
	.globl	_start
	_start:
		li		a7, 3				# mem_alloc
		li		a0, 0xffff0000		# address
		li		a1, 0x10000 / 4096	# size (64K)
		li		a2, 0b011			# flags (RW)
		ecall

	0:
		bnez	a0, 0b				# Loop forever on error

		li		sp, 0xffffffff			# Set stack pointer

		addi	sp, sp, -8			# Set return address to 0 to aid debugger
		sd		zero, 0(sp)

		call	main

	0:
		j		0b				# Loop forever as we can't exit
	",
);

const PCI_ADDRESS: NonNull<kernel::Page> = unsafe { NonNull::new_unchecked(0x10_0000_0000 as *mut _) };
const PCI_ADDRESS_IO: NonNull<kernel::Page> = unsafe { NonNull::new_unchecked(0x20_0000_0000 as *mut _) };
const PCI_ADDRESS_MMIO_A: NonNull<kernel::Page> = unsafe { NonNull::new_unchecked(0x30_0000_0000 as *mut _) };
const PCI_ADDRESS_MMIO_B: NonNull<kernel::Page> = unsafe { NonNull::new_unchecked(0x40_0000_0000 as *mut _) };
const VIRTIO_VENDOR_ID: u16 = 0x1af4;
const VIRTIO_DEV_BLK_ID: u16 = 0x1001;

#[export_name = "main"]
fn main() {
	let kernel::Return { status, value } = unsafe {
		let ranges = [PCI_ADDRESS_IO.as_ptr(), PCI_ADDRESS_MMIO_A.as_ptr(), PCI_ADDRESS_MMIO_B.as_ptr()];
		kernel::dev_reserve(
			0,
			PCI_ADDRESS.as_ptr(),
			&ranges as *const _,
			ranges.len(),
		)
	};
	assert_eq!(status, 0, "Failed to reserve device");

	// TODO don't hardcode the PCI device info ya fucking doof.
	let pci = unsafe {
		pci::PCI::new(
			PCI_ADDRESS,
			256 * 32 * 8 * 4096,
			&[
				pci::PhysicalMemory {
					virt: NonNull::new_unchecked(PCI_ADDRESS_MMIO_A.as_ptr()),
					physical: 0x4000_0000,
					size: 0x4000_0000,
				},
				pci::PhysicalMemory {
					virt: NonNull::new_unchecked(PCI_ADDRESS_MMIO_A.as_ptr()),
					physical: 0x4_000_0000,
					size: 0x4_000_0000,
				},
			],
		)
	};

	for dev in pci.iter().flat_map(|b| b.iter()) {
		writeln!(kernel::SysLog, "Vendor: {:x}", dev.vendor_id());
		if dev.vendor_id() == VIRTIO_VENDOR_ID {
			writeln!(kernel::SysLog, "Device: {:x}", dev.device_id());
			match dev.device_id() {
				VIRTIO_DEV_BLK_ID => {
					let dev = virtio::pci::new_device::<GlobalAlloc>(dev)
						.expect("Failed to setup virtio device");
					// BE FUCKING ALIGNED OR SMTH I GUESS?
					#[repr(align(4096))]
					struct FUCK {
						data: [u8; 512],
					}
					let mut data = FUCK { data: [0; 512] };
					let mut data = &mut data.data;
					writeln!(kernel::SysLog, "{:?}", data as *mut _);
					for (s, d) in b"Hello, world! I am a userland driver!".iter().copied().zip(data.iter_mut()) {
						write!(kernel::SysLog, "{}", s as char);
						*d = s;
					}
					writeln!(kernel::SysLog, "");
					if let virtio::pci::Device::Block(mut dev) = dev {
						dev
							.write(data, 0)
							.expect("Failed to write");
					} else {
						unreachable!();
					}
				}
				_ => (),
			}
		}
	}
}

use core::alloc::{Allocator, AllocError, Layout};

struct GlobalAlloc;

unsafe impl Allocator for GlobalAlloc {
	fn allocate(&self, _: Layout) -> Result<NonNull<[u8]>, AllocError> { todo!() }
	unsafe fn deallocate(&self, _: NonNull<u8>, _: Layout) { todo!() }
}
