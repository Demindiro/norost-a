//! Virtual-Memory Systems implementations
//!
//! ## References
//!
//! [RISC-V Priviliged Architecture][rv], chapters 4.3, 4.4 and 4.5
//!
//! [rv]: https://riscv.org/wp-content/uploads/2017/05/riscv-privileged-v1.10.pdf

mod sv39;

use crate::arch::vms::RWX;

pub use sv39::Sv39;

/// Convert RWX flags to a format suitable for Leaf entries
fn from_rwx(rwx: RWX) -> u32 {
	match rwx {
		RWX::R => 0b0010,
		RWX::RW => 0b0110,
		RWX::X => 0b1000,
		RWX::RX => 0b1010,
		RWX::RWX => 0b1110,
	}
}

/// Convert encoded flags to RWX
fn to_rwx(flags: u64) -> Option<RWX> {
	Some(match flags & 0xe {
		0b0010 => RWX::R,
		0b0110 => RWX::RW,
		0b1000 => RWX::X,
		0b1010 => RWX::RX,
		0b1110 => RWX::RWX,
		_ => return None,
	})
}
