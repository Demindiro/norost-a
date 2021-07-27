//! # Mapped pages
//!
//! To differentiate between pages with certain accessibilities, `Map` is used.
//! A `Map` can be private, shared, shared & locked or direct.
//!
//! * A private map is tracked by the physical memory manager. There may be only one
//!  reference to such a page at any time.
//!
//! * A shared map is also tracked by the physical memory manager, but there may be
//!   multiple references to it at any time, as it is reference counted. It may also
//!   have a "locked" hint, which indicates whether certain attributes should be able
//!   to change (e.g. RWX flags).
//!
//! * A direct map is not tracked by the physical memory manager. Instead, the process
//!   mapped it directly into its address space. This is normally only used for special
//!   addresses such as MMIO.

use crate::memory::ppn::*;

/// A PPN of a certain type.
#[repr(u8)]
pub enum Map {
	Private(PPN) = 0b00,
	Direct(PPNDirect) = 0b01,
	Shared(SharedPPN) = 0b10,
	SharedLocked(SharedPPN) = 0b11,
}

impl Map {}

/// A range of PPNs of a certain type.
#[repr(u8)]
pub enum MapRange {
	Private(PPNRange) = 0b00,
	Direct(PPNDirectRange) = 0b01,
	Shared(SharedPPNRange) = 0b10,
	SharedLocked(SharedPPNRange) = 0b11,
}

impl MapRange {
	pub fn len(&self) -> usize {
		match self {
			Self::Private(p) => p.len(),
			Self::Direct(d) => d.len(),
			Self::Shared(s) | Self::SharedLocked(s) => s.len(),
		}
	}

	pub fn start(&self) -> PPNBox {
		match self {
			Self::Private(p) => p.start(),
			Self::Direct(d) => d.start(),
			Self::Shared(s) | Self::SharedLocked(s) => s.start(),
		}
	}

	pub fn pop_base(&mut self) -> Option<Map> {
		match self {
			Self::Private(p) => p.pop_base().map(Map::Private),
			Self::Direct(d) => d.pop_base().map(Map::Direct),
			Self::Shared(s) => s.pop_base().map(Map::Shared),
			Self::SharedLocked(s) => s.pop_base().map(Map::SharedLocked),
		}
	}

	pub fn forget_base(&mut self, count: usize) -> usize {
		match self {
			Self::Private(p) => p.forget_base(count),
			Self::Direct(d) => d.forget_base(count),
			Self::Shared(s) | Self::SharedLocked(s) => s.forget_base(count),
		}
	}
}
