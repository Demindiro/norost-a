//! # Device tree parsing.
//!
//! This module keeps track of used devices & their addresses.

use core::convert::{TryFrom, TryInto};
use core::num::NonZeroUsize;
use core::ptr::NonNull;

pub struct Device<'a> {
	pub name: &'a [u8],
	pub compatible: &'a [&'a [u8]],
	pub reg: &'a [driver::Reg],
	pub ranges: &'a [driver::Range],
	pub interrupt_map: &'a [driver::InterruptMap],
	pub interrupt_map_mask: driver::InterruptMapMask,
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
					let mut child_interrupt_cells = 0;
					let mut raw_compatible = &[][..];
					let mut raw_ranges = &[][..];
					let mut raw_reg = &[][..];
					let mut raw_interrupt_map = &[][..];

					for p in node.properties() {
						match p.name {
							b"compatible" => raw_compatible = p.value,
							b"interrupt-map" => raw_interrupt_map = p.value,
							b"ranges" => raw_ranges = p.value,
							b"reg" => raw_reg = p.value,
							b"#address-cells" => {
								child_address_cells =
									u32::from_be_bytes(p.value.try_into().unwrap())
							}
							b"#interrupt-cells" => {
								child_interrupt_cells =
									u32::from_be_bytes(p.value.try_into().unwrap())
							}
							_ => (),
						}
					}

					let name = node.name.split(|c| *c == b'@').next().unwrap();

					// Parse reg
					let mut addr_size = [driver::Reg::new(0, 0); 8];
					let mut reg = raw_reg;
					let mut as_i = 0;

					while !reg.is_empty() {
						let (a, r) = unpack_reg(reg, node.address_cells);
						let (s, r) = unpack_reg(r, node.size_cells);
						addr_size[as_i] = driver::Reg::new(a, s);
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
					let mut ranges = [driver::Range::new(0, 0, 0); 8];
					let mut r_i = 0;

					while !raw_ranges.is_empty() {
						let (c, r) = unpack_reg(raw_ranges, child_address_cells);
						let (a, r) = unpack_reg(r, node.address_cells);
						let (s, r) = unpack_reg(r, node.size_cells);
						ranges[r_i] = driver::Range::new(c, a, s);
						raw_ranges = r;
						r_i += 1;
					}

					// Parse interrupt map
					let mut interrupt_map = [driver::InterruptMap::new(0, 0, 0, 0, 0); 32];
					let mut im_i = 0;

					// FIXME retrieve this from the actual interrupt controller
					let parent_address_cells = 0;
					let parent_interrupt_cells = 1;

					while !raw_interrupt_map.is_empty() {
						let (ca, r) = unpack_reg(raw_interrupt_map, child_address_cells);
						let (ci, r) = unpack_reg(r, child_interrupt_cells);
						let (ph, r) = unpack_reg(r, 1);
						let (pa, r) = unpack_reg(r, parent_address_cells);
						let (pi, r) = unpack_reg(r, parent_interrupt_cells);
						let ph = ph.try_into().unwrap();
						interrupt_map[im_i] = driver::InterruptMap::new(ca, ci, ph, pa, pi);
						im_i += 1;
						raw_interrupt_map = r;
					}

					let interrupt_map_mask = if !interrupt_map[..im_i].is_empty() {
						let raw_interrupt_map_mask = node
							.properties()
							.find(|p| p.name == b"interrupt-map-mask")
							.map(|p| p.value)
							.unwrap();
						let (child_address, r) =
							unpack_reg(raw_interrupt_map_mask, child_address_cells);
						let (child_interrupt, r) = unpack_reg(r, child_interrupt_cells);
						driver::InterruptMapMask {
							child_address,
							child_interrupt,
						}
					} else {
						driver::InterruptMapMask {
							child_address: 0,
							child_interrupt: 0,
						}
					};

					f(Device {
						name,
						reg: &addr_size[..as_i],
						compatible: &compatible[..c_i],
						ranges: &ranges[..r_i],
						interrupt_map: &interrupt_map[..im_i],
						interrupt_map_mask,
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
