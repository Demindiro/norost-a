//! Virtual-Memory Systems implementations
//!
//! ## References
//!
//! [RISC-V Priviliged Architecture][rv], chapters 4.3, 4.4 and 4.5
//!
//! [rv]: https://riscv.org/wp-content/uploads/2017/05/riscv-privileged-v1.10.pdf

mod sv39;

pub use sv39::Sv39;

use crate::memory::AllocateError;
use core::convert::TryFrom;

/// Valid RWX flag combinations
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RWX {
	R = 0b0010,
	RW = 0b0110,
	X = 0b1000,
	RX = 0b1010,
	RWX = 0b1110,
}

impl RWX {
	pub fn r(self) -> bool {
		self == Self::R || self == Self::RW || self == Self::RX || self == Self::RWX
	}

	pub fn w(self) -> bool {
		self == Self::RW || self == Self::RWX
	}

	pub fn x(self) -> bool {
		self == Self::X || self == Self::RX || self == Self::RWX
	}
}

impl From<RWX> for u32 {
	fn from(rwx: RWX) -> Self {
		match rwx {
			RWX::R => 0b0010,
			RWX::RW => 0b0110,
			RWX::X => 0b1000,
			RWX::RX => 0b1010,
			RWX::RWX => 0b1110,
		}
	}
}

impl From<RWX> for u64 {
	fn from(rwx: RWX) -> Self {
		u32::from(rwx).into()
	}
}

impl TryFrom<u64> for RWX {
	type Error = ();

	fn try_from(rwx: u64) -> Result<Self, Self::Error> {
		match rwx {
			0b0010 => Ok(Self::R),
			0b0110 => Ok(Self::RW),
			0b1000 => Ok(Self::X),
			0b1010 => Ok(Self::RX),
			0b1110 => Ok(Self::RWX),
			_ => Err(()),
		}
	}
}

impl TryFrom<u32> for RWX {
	type Error = ();

	fn try_from(rwx: u32) -> Result<Self, Self::Error> {
		Self::try_from(rwx as u64)
	}
}

/// Possible errors when adding a mapping
#[derive(Debug)]
pub enum AddError {
	/// The mapping overlaps with an existing mapping
	Overlaps,
	OutOfRange,
	AllocateError(AllocateError),
}

/// Possible errors when sharing a mapping
#[derive(Debug)]
pub enum ShareError {
	/// The mapping overlaps with an existing mapping
	Overlaps,
	OutOfRange,
	AllocateError(AllocateError),
	NoEntry,
}

impl From<AddError> for ShareError {
	fn from(error: AddError) -> Self {
		match error {
			AddError::Overlaps => Self::Overlaps,
			AddError::OutOfRange => Self::OutOfRange,
			AddError::AllocateError(e) => Self::AllocateError(e),
		}
	}
}
