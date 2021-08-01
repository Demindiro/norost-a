//! # Device tree parsing.
//!
//! This module keeps track of used devices & their addresses.

use core::convert::{TryFrom, TryInto};
use core::ptr::NonNull;

pub const UART_ADDRESS: NonNull<u8> = unsafe { NonNull::new_unchecked(0x1000_0000 as *mut _) };

pub fn map_devices() {
	let dtb = unsafe {
		let dtb = 0x100_0000 as *mut _;
		let kernel::Return {
			status,
			value: count,
		} = kernel::sys_platform_info(dtb, 16);
		core::slice::from_raw_parts(dtb.cast(), count << 10)
	};
	let dtb = device_tree::DeviceTree::parse(dtb).unwrap();
	if let Ok(node) = dtb.root() {
		for node in node.children() {
			if node.name == b"soc" {
				for node in node.children() {
					let mut ranges = None;
					let mut reg = None;
					for p in node.properties() {
						match p.name {
							b"ranges" => ranges = Some(p.value),
							b"reg" => reg = Some(p.value),
							_ => (),
						}
					}

					match node.name.split(|c| *c == b'@').next().unwrap() {
						b"uart" => {
							let reg = reg.unwrap();
							let (addr, reg) = unpack_reg(reg, node.address_cells);
							let (size, reg) = unpack_reg(reg, node.address_cells);
							assert!(reg.is_empty());
							let kernel::Return { status, .. } = unsafe {
								kernel::sys_direct_alloc(
									UART_ADDRESS.as_ptr().cast(),
									(addr >> 12).try_into().unwrap(),
									((size + 0xfff) >> 12).try_into().unwrap(),
									0b011,
								)
							};
							assert_eq!(status, 0, "mapping UART failed");
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
