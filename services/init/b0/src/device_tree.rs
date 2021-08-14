//! # Device tree parsing.
//!
//! This module keeps track of used devices & their addresses.

use core::convert::{TryFrom, TryInto};
use core::ptr::NonNull;

pub const PCI_ADDRESS: NonNull<u8> = unsafe { NonNull::new_unchecked(0x2000_0000 as *mut _) };
pub const PCI_MMIO_ADDRESS: NonNull<u8> = unsafe { NonNull::new_unchecked(0x3000_0000 as *mut _) };
pub static mut PCI_MMIO_PHYSICAL: (usize, usize) = (0, 0);
pub static mut PCI_SIZE: usize = 0;
pub const PLIC_ADDRESS: NonNull<u32> = unsafe { NonNull::new_unchecked(0x4_0000_0000 as *mut _) };

pub fn map_devices() {
	let dtb = unsafe {
		let dtb = 0x100_0000 as *mut _;
		let ret = kernel::sys_platform_info(dtb, 16);
		assert_eq!(ret.status, 0);
		core::slice::from_raw_parts(dtb.cast(), ret.value << 10)
	};
	let dtb = device_tree::DeviceTree::parse(dtb).unwrap();
	kernel::sys_log!("Iterating DTB");
	if let Ok(node) = dtb.root() {
		for node in node.children() {
			if node.name == b"soc" {
				for node in node.children() {
					//let mut ranges = None;
					let mut reg = None;
					for p in node.properties() {
						match p.name {
							//b"ranges" => ranges = Some(p.value),
							b"reg" => reg = Some(p.value),
							_ => (),
						}
					}

					match node.name.split(|c| *c == b'@').next().unwrap() {
						b"pci" => {
							let mut compatible = false;
							let mut ranges = None;
							let mut reg = None;
							let mut child_address_cells = None;
							for p in node.properties() {
								match p.name {
									b"compatible" => {
										compatible = p.value == b"pci-host-ecam-generic\0"
									}
									b"ranges" => ranges = Some(p.value),
									b"reg" => reg = Some(p.value),
									b"#address-cells" => {
										child_address_cells =
											Some(u32::from_be_bytes(p.value.try_into().unwrap()))
									}
									_ => (),
								}
							}
							if !compatible {
								continue;
							}

							kernel::sys_log!("  Found PCI");

							// Map regions into address space.

							// Map reg
							let addr = PCI_ADDRESS.cast();
							let reg = reg.expect("No reg property");
							let (start, reg) = unpack_reg(reg, node.address_cells);
							let (size, _) = unpack_reg(reg, node.size_cells);
							let (start, size) = (
								usize::try_from(start).unwrap(),
								usize::try_from(size).unwrap(),
							);
							let ret = unsafe {
								kernel::sys_direct_alloc(
									addr.as_ptr(),
									start >> 12,
									size >> 12,
									0b011,
								)
							};
							assert_eq!(ret.status, 0);
							unsafe { PCI_SIZE = size };
							kernel::sys_log!(
								"    Address {:p} -> 0x{:x} - 0x{:x}",
								addr,
								start,
								size
							);

							// Map MMIO
							let addr = PCI_MMIO_ADDRESS.cast();
							let ranges = &ranges.unwrap()[(child_address_cells.unwrap()
								+ node.address_cells + node.size_cells)
								as usize * 4..];
							let r = &ranges[child_address_cells.unwrap() as usize * 4..];
							let (start, r) = unpack_reg(r, node.address_cells);
							let (size, _) = unpack_reg(r, node.size_cells);
							let (start, size) = (
								usize::try_from(start).unwrap(),
								usize::try_from(size).unwrap(),
							);
							let ret = unsafe {
								kernel::sys_direct_alloc(
									addr.as_ptr(),
									start >> 12,
									size >> 12,
									0b011,
								)
							};
							assert_eq!(ret.status, 0);
							kernel::sys_log!(
								"    MMIO    {:p} -> 0x{:x} - 0x{:x}",
								addr,
								start,
								size
							);
							unsafe { PCI_MMIO_PHYSICAL = (start, size) };
						}
						b"uart" => {
							kernel::sys_log!("  Found UART");
							let reg = reg.unwrap();
							let (addr, reg) = unpack_reg(reg, node.address_cells);
							let (size, reg) = unpack_reg(reg, node.size_cells);
							assert!(reg.is_empty());
							// TODO design a protocol for communicating the address with the driver
						}
						b"plic" => {
							let reg = reg.unwrap();
							let (addr, reg) = unpack_reg(reg, node.address_cells);
							let (size, reg) = unpack_reg(reg, node.size_cells);
							assert!(reg.is_empty());
							let ret = unsafe {
								let addr = addr >> kernel::Page::OFFSET_BITS;
								let size = (size + u128::try_from(kernel::Page::MASK).unwrap())
									>> kernel::Page::OFFSET_BITS;
								kernel::sys_direct_alloc(
									PLIC_ADDRESS.as_ptr().cast(),
									addr.try_into().unwrap(),
									usize::try_from(size).unwrap(),
									0b011,
								)
							};
							assert_eq!(ret.status, 0, "mapping PLIC failed");

							let max_devices = node
								.properties()
								.find(|p| p.name == b"riscv,ndev")
								.expect("missing property riscv,ndev");
							let (max_devices, _) = unpack_reg(max_devices.value, 1);

							let ret = unsafe {
								let addr = addr >> kernel::Page::OFFSET_BITS;
								let size = (size + u128::try_from(kernel::Page::MASK).unwrap())
									>> kernel::Page::OFFSET_BITS;
								kernel::sys_set_interrupt_controller(
									addr.try_into().unwrap(),
									usize::try_from(size).unwrap(),
									max_devices as u16,
								)
							};
							assert_eq!(ret.status, 0, "registering PLIC failed");
						}
						_ => (),
					}
				}
			}
		}
	}
}

#[track_caller]
fn unpack_reg(reg: &[u8], cells: u32) -> (u128, &[u8]) {
	assert!(cells <= 4, "unsupported cells size: {}", cells);
	let cells = usize::try_from(cells).unwrap() * 4;
	assert!(cells <= reg.len(), "reg isn't large enough");
	let mut num = 0;
	for i in 0..cells {
		num = (num << 8) | u128::from(reg[i]);
	}
	(num, &reg[cells..])
}
