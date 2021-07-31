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

	fn print_node(level: usize, mut node: device_tree::Node) {
		writeln!(kernel::SysLog, "{0:>1$}{2} {{", "", level * 2, node.name);
		while let Some(property) = node.next_property() {
			if property.value.len() > 0
				&& property.value[..property.value.len() - 1]
					.iter()
					// Everything between ' ' and '~' is human-readable
					.all(|&c| b' ' <= c && c <= b'~')
				&& property.value.last().unwrap() == &0
			{
				// SAFETY: The string is a valid null-terminated string
				let s = unsafe {
					core::str::from_utf8_unchecked(&property.value[..property.value.len() - 1])
				};
				writeln!(
					kernel::SysLog,
					"{0:>1$}{2} = {3:?}",
					"",
					level * 2 + 2,
					property.name,
					s
				);
			} else {
				writeln!(
					kernel::SysLog,
					"{0:>1$}{2} = {3:02x?}",
					"",
					level * 2 + 2,
					property.name,
					&property.value
				);
			}
		}
		while let Some(node) = node.next_child_node() {
			print_node(level + 1, node);
		}
		writeln!(kernel::SysLog, "{0:>1$}}}", "", level * 2);
	}
	let mut interpreter = dtb.interpreter();
	while let Some(mut node) = interpreter.next_node() {
		print_node(1, node);
	}
}

fn map_pci(dtb: device_tree::DeviceTree) {
	let mut int = dtb.interpreter();

	let mut address_cells = 0;
	let mut size_cells = 0;

	while let Some(mut node) = int.next_node() {
		while let Some(p) = node.next_property() {
			match p.name {
				"#address-cells" => {
					let num = p.value.try_into().expect("Malformed #address-cells");
					address_cells = u32::from_be_bytes(num);
				}
				"#size-cells" => {
					let num = p.value.try_into().expect("Malformed #size-cells");
					size_cells = u32::from_be_bytes(num);
				}
				_ => (),
			}
		}

		while let Some(mut node) = node.next_child_node() {
			if node.name == "soc" {
				while let Some(p) = node.next_property() {}
				while let Some(mut node) = node.next_child_node() {
					let mut compatible = false;
					let mut ranges = None;
					let mut reg = None;
					let mut child_address_cells = address_cells;
					let mut child_size_cells = size_cells;
					while let Some(p) = node.next_property() {
						match p.name {
							"compatible" => compatible = p.value == b"pci-host-ecam-generic\0",
							"ranges" => ranges = Some(p.value),
							"reg" => reg = Some(p.value),
							"#address-cells" => {
								child_address_cells =
									u32::from_be_bytes(p.value.try_into().unwrap())
							}
							"#size-cells" => {
								child_size_cells = u32::from_be_bytes(p.value.try_into().unwrap())
							}
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
					let (start, reg): (usize, _) = match address_cells {
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
					let size: usize = match size_cells {
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
						[(child_address_cells + address_cells + size_cells) as usize * 4..];
					let r = &ranges[child_address_cells as usize * 4..];
					let (start, r): (usize, _) = match address_cells {
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
					let (size, r): (usize, _) = match size_cells {
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
		let d = device_tree::DeviceTree::parse_dtb(PLATFORM_INFO.as_ptr().cast()).unwrap();
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
