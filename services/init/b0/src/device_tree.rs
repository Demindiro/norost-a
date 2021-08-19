//! # Device tree parsing.
//!
//! This module keeps track of used devices & their addresses.

use core::convert::{TryFrom, TryInto};
use core::num::NonZeroUsize;
use core::ptr::NonNull;

pub struct Device<'a> {
	pub name: &'a [u8],
	pub compatible: &'a [&'a [u8]],
	pub reg: &'a [(u128, u128)],
	pub ranges: &'a [(u128, u128, u128)],
}

pub fn iter_devices<F>(mut f: F)
where
	F: FnMut(Device),
{
	let dtb = unsafe {
		let dtb = 0x100_0000 as *mut _;
		let ret = kernel::sys_platform_info(dtb, 16);
		assert_eq!(ret.status, 0);
		core::slice::from_raw_parts(dtb.cast(), ret.value << 10)
	};

	let dtb = device_tree::DeviceTree::parse(dtb).unwrap();

	if let Ok(node) = dtb.root() {
		for node in node.children() {
			if node.name == b"soc" {
				for node in node.children() {
					let mut child_address_cells = 0;
					let mut raw_compatible = &[][..];
					let mut raw_ranges = &[][..];
					let mut raw_reg = &[][..];

					for p in node.properties() {
						match p.name {
							b"compatible" => raw_compatible = p.value,
							b"ranges" => raw_ranges = p.value,
							b"reg" => raw_reg = p.value,
							b"#address-cells" => {
								child_address_cells =
									u32::from_be_bytes(p.value.try_into().unwrap())
							}
							_ => (),
						}
					}

					let name = node.name.split(|c| *c == b'@').next().unwrap();

					// Parse reg
					let mut addr_size = [(0, 0); 32];
					let mut reg = raw_reg;
					let mut as_i = 0;

					while !reg.is_empty() {
						let (a, r) = unpack_reg(reg, node.address_cells);
						let (s, r) = unpack_reg(r, node.size_cells);
						addr_size[as_i] = (a, s);
						reg = r;
						as_i += 1;
					}

					// Parse compatible
					let mut compatible = [&[][..]; 8];
					let mut c_i = 0;
					for (i, s) in raw_compatible.split(|c| c == &b'\0').enumerate() {
						compatible[i] = s;
						c_i = i;
					}

					// Parse ranges
					let mut ranges = [(0, 0, 0); 8];
					let mut r_i = 0;

					while !raw_ranges.is_empty() {
						let (c, r) = unpack_reg(raw_ranges, child_address_cells);
						let (a, r) = unpack_reg(r, node.address_cells);
						let (s, r) = unpack_reg(r, node.size_cells);
						ranges[r_i] = (c, a, s);
						raw_ranges = r;
						r_i += 1;
					}

					f(Device {
						name,
						reg: &addr_size[..as_i],
						compatible: &compatible[..c_i],
						ranges: &ranges[..r_i],
					});
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
