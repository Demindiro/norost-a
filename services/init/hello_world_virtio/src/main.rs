#![no_std]
#![no_main]
#![feature(allocator_api)]
#![feature(global_asm)]
#![feature(panic_info_message)]
#![feature(ptr_metadata)]

use core::convert::TryInto;
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

global_asm!(
	"
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

const PLATFORM_INFO: NonNull<kernel::Page> =
	unsafe { NonNull::new_unchecked(0x10_0000_0000 as *mut _) };
const PCI_ADDRESS: NonNull<kernel::Page> =
	unsafe { NonNull::new_unchecked(0x20_0000_0000 as *mut _) };
const PCI_ADDRESS_MMIO: NonNull<kernel::Page> =
	unsafe { NonNull::new_unchecked(0x30_0000_0000 as *mut _) };
const VIRTIO_VENDOR_ID: u16 = 0x1af4;
const VIRTIO_DEV_BLK_ID: u16 = 0x1001;

fn dump_dtb(dtb: &device_tree::DeviceTree) {
	writeln!(kernel::SysLog, "Device tree:");
	writeln!(kernel::SysLog, "  Reserved memory regions:");
	for rmr in dtb.reserved_memory_regions() {
		let addr = u64::from(rmr.address) as usize;
		let size = u64::from(rmr.size) as usize;
		writeln!(
			kernel::SysLog,
			"  {:x} <-> {:x} (len: {:x})",
			addr,
			addr + size,
			size
		);
	}

	kernel::sys_log!("{:#?}", dtb);
}

fn map_pci(dtb: device_tree::DeviceTree) {
	if let Ok(node) = dtb.root() {
		for node in node.children() {
			if node.name == b"soc" {
				for node in node.children() {
					let mut compatible = false;
					let mut ranges = None;
					let mut reg = None;
					let mut child_address_cells = None;
					for p in node.properties() {
						match p.name {
							b"compatible" => compatible = p.value == b"pci-host-ecam-generic\0",
							b"ranges" => ranges = Some(p.value),
							b"reg" => reg = Some(p.value),
							b"#address-cells" => child_address_cells = Some(u32::from_be_bytes(p.value.try_into().unwrap())),
							_ => (),
						}
					}
					if !compatible {
						continue;
					}

					// Map regions into address space.

					// Map reg
					let addr = PCI_ADDRESS;
					let reg = reg.expect("No reg property");
					let (start, reg): (usize, _) = match node.address_cells {
						1 => (
							u32::from_be_bytes(reg[..4].try_into().unwrap())
								.try_into()
								.unwrap(),
							&reg[4..],
						),
						2 => (
							u64::from_be_bytes(reg[..8].try_into().unwrap())
								.try_into()
								.unwrap(),
							&reg[8..],
						),
						_ => panic!("Address cell size too large"),
					};
					let size: usize = match node.size_cells {
						1 => u32::from_be_bytes(reg.try_into().unwrap())
							.try_into()
							.unwrap(),
						2 => u64::from_be_bytes(reg.try_into().unwrap())
							.try_into()
							.unwrap(),
						_ => panic!("Size cell size too large"),
					};
					unsafe {
						kernel::sys_direct_alloc(addr.as_ptr(), start >> 12, size >> 12, 0b111);
					}

					// Map MMIO
					let addr = PCI_ADDRESS_MMIO;
					let ranges = &ranges.unwrap()
						[(child_address_cells.unwrap() + node.address_cells + node.size_cells) as usize * 4..];
					let r = &ranges[child_address_cells.unwrap() as usize * 4..];
					let (start, r): (usize, _) = match node.address_cells {
						1 => (
							u32::from_be_bytes(r[..4].try_into().unwrap())
								.try_into()
								.unwrap(),
							&r[4..],
						),
						2 => (
							u64::from_be_bytes(r[..8].try_into().unwrap())
								.try_into()
								.unwrap(),
							&r[8..],
						),
						_ => panic!("Address cell size too large"),
					};
					let (size, r): (usize, _) = match node.size_cells {
						1 => (
							u32::from_be_bytes(r[..4].try_into().unwrap())
								.try_into()
								.unwrap(),
							&r[4..],
						),
						2 => (
							u64::from_be_bytes(r[..8].try_into().unwrap())
								.try_into()
								.unwrap(),
							&r[8..],
						),
						_ => panic!("Size cell size too large"),
					};
					unsafe {
						kernel::sys_direct_alloc(addr.as_ptr(), start >> 12, size >> 12, 0b111);
					}
				}
			}
		}
	}
}

#[export_name = "main"]
fn main() {
	let kernel::Return { status, value } =
		unsafe { kernel::sys_platform_info(PLATFORM_INFO.as_ptr(), 256) };

	assert_eq!(status, 0, "Failed to get platform info");
	writeln!(kernel::SysLog, "ergijoregiogreojr");

	unsafe {
		let d = core::slice::from_raw_parts(PLATFORM_INFO.as_ptr().cast(), 4096 / 4 * value);
		let d = device_tree::DeviceTree::parse(d).unwrap();
		#[cfg(feature = "dump-dtb")]
		dump_dtb(&d);
		map_pci(d);
	}

	assert_eq!(status, 0, "Failed to reserve device");

	// TODO don't hardcode the PCI device info ya fucking doof.
	let pci = unsafe {
		pci::PCI::new(
			PCI_ADDRESS,
			256 * 32 * 8 * 4096,
			&[pci::PhysicalMemory {
				virt: NonNull::new_unchecked(PCI_ADDRESS_MMIO.as_ptr()),
				physical: 0x4000_0000,
				size: 0x4000_0000,
			}],
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
					for (s, d) in
						b"Hello, world! I am a userland driver!\nLook at me, I parse the FDT now!"
							.iter()
							.copied()
							.zip(data.iter_mut())
					{
						write!(kernel::SysLog, "{}", s as char);
						*d = s;
					}
					writeln!(kernel::SysLog, "");
					if let virtio::pci::Device::Block(mut dev) = dev {
						dev.write(data, 0).expect("Failed to write");
					} else {
						unreachable!();
					}
				}
				_ => (),
			}
		}
	}
}

use core::alloc::{AllocError, Allocator, Layout};

struct GlobalAlloc;

unsafe impl Allocator for GlobalAlloc {
	fn allocate(&self, _: Layout) -> Result<NonNull<[u8]>, AllocError> {
		todo!()
	}
	unsafe fn deallocate(&self, _: NonNull<u8>, _: Layout) {
		todo!()
	}
}
