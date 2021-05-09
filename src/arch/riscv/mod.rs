//! Helper methods and structures for RISC-V specific information and control.
//!
//! ## References
//!
//! [Volume I: Unprivileged ISA][spec]
//! [Volume II: Privileged Architecture][priv]
//!
//! [spec]: https://github.com/riscv/riscv-isa-manual/releases/download/Ratified-IMAFDQC/riscv-spec-20191213.pdf
//! [priv]: https://github.com/riscv/riscv-isa-manual/releases/download/Ratified-IMFDQC-and-Priv-v1.11/riscv-privileged-20190608.pdf

pub mod rv64;

use crate::{log, util};
use core::{array, mem};

/// A wrapper around the contents of the `misa` register, which describes the base ISA and it's
/// supported extensions.
pub struct MISA;

/// An enum indicating the width of the ISA
pub enum MXL {
	XLEN32 = 1,
	XLEN64 = 2,
	XLEN128 = 3,
}

/// An enum that can be used to check for extension flags.
///
/// ## Notes
///
/// While it is alphabetical and it is possible to just accept a `u8` and do `c - b'a'`, it would
/// force us to add error checking. By using an enum no error checking is needed at all.
pub enum Extension {
	A = 0,
	B = 1,
	C = 2,
	D = 3,
	E = 4,
	F = 5,
	G = 6,
	H = 7,
	I = 8,
	J = 9,
	K = 10,
	L = 11,
	M = 12,
	N = 13,
	O = 14,
	P = 15,
	Q = 16,
	R = 17,
	S = 18,
	T = 19,
	U = 20,
	V = 21,
	W = 22,
	X = 23,
	Y = 24,
	Z = 25,
}

impl MISA {
	/// The address of the `misa` register
	const CSR: u16 = 0x301;

	/// Wraps the contents of the `misa` register. This constructor is a NOOP, though
	/// this may change.
	#[cold]
	pub fn new() -> Self {
		Self
	}

	/// Returns the base integer width of this ISA.
	#[cold]
	#[must_use]
	pub fn mxl(&self) -> MXL {
		// Quoting the RISCV manual:
		//
		// "The base width can be quickly ascertained using branches on the sign of the returned
		// `misa` value, and possibly a shift left by one and a second branch on the sign. These
		// checks can be written in assembly code without knowing the register width (XLEN) of the
		// machine. The base width is given by XLEN = 2^(MXL + 4)."
		//
		// "The base width can also be found if `misa` is zero, by placing the immediate 4 in a
		// register then shifting the register left by 31 bits at a time. If zero after one shift,
		// then the machine is RV32. If zero after two shifts, then the machine is RV64, else
		// RV128."

		// SAFETY: Inspecting `misa` is always safe.
		unsafe {
			let mxl: u8;
			asm!("
				csrrs	a0, {csr}, zero
				ble		a0, zero, none
				bgt		a0, zero, 32f
				slli	a0, a0, 1
				bgt		a0, zero, 64f
				j		128f

			none:
				li		a0, 4
				slli	a0, a0, 31
				ble		a0, zero, 32f
				slli	a0, a0, 31
				ble		a0, zero, 64f
				j		128f

			32:
				li		a0, 0
				j		done
			64:
				li		a0, 1
				j		done
			128:
				li		a0, 2

			done:
			",
				csr = const Self::CSR,
				out("a0") mxl,
			);
			match mxl {
				0 => MXL::XLEN32,
				1 => MXL::XLEN64,
				2 => MXL::XLEN128,
				_ => unreachable!(),
			}
		}
	}

	/// Returns `true` if the given extension is present, otherwise false
	#[cold]
	#[must_use]
	pub fn has_extension(&self, extension: Extension) -> bool {
		let misa: usize;
		// SAFETY: Inspecting `misa` is always safe.
		unsafe {
			asm!("csrrs	a0, {csr}, zero", csr = const Self::CSR, out("a0") misa);
		}
		misa & (1 << extension as u8) != 0
	}

	/// Logs the value of `misa` in human-readable format
	#[cold]
	pub fn log(&self) {
		let mut buf = [0; 64];
		let misa: usize;
		// SAFETY: Inspecting `misa` is always safe
		unsafe {
			asm!("csrrs	a0, {csr}, zero", csr = const Self::CSR, out("a0") misa);
		}
		let misa = util::usize_to_string_hex(&mut buf, misa).unwrap();
		log::info(&["MISA = ", misa]);
		log::info(&["  MXL = ", self.mxl().as_str()]);
		{
			use Extension::*;
			log::info(&["  Extensions:"]);
			let e = [
				A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
			];
			for e in array::IntoIter::new(e) {
				let s = e.as_str();
				if self.has_extension(e) {
					log::info(&["    ", s]);
				}
			}
		}
	}
}

impl MXL {
	/// Returns the MXL as a human-readable string
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::XLEN32 => "32",
			Self::XLEN64 => "64",
			Self::XLEN128 => "128",
		}
	}
}

/// A macro to generate the match table for human-readable extensions
/// This is implemented as a macro to make it easy to reduce the size
/// of the strings (useful for extremely resource constrained environments).
macro_rules! human_extension {
	{ $($ext:ident $description:literal)* } => {
		impl Extension {
			/// Returns the extension as a human-readable string
			pub fn as_str(&self) -> &'static str {
				match self {
					$(Self::$ext => concat!(stringify!($ext), " (", $description, ")"),)*
				}
			}
		}
	};
}

human_extension! {
	A "Atomics"
	B "Bit manipulation"
	C "Compressed"
	D "Double precision floating point"
	E "RV32E base ISA"
	F "Single precision floating point"
	G "Additional standard extensions"
	H "Hypervisor"
	I "RV32I/64I/128I base ISA"
	J "Dynamically translated languages"
	K "RESERVED"
	L "Decimal floating point"
	M "Integer multiplication & division"
	N "User-level interrupts"
	O "RESERVED"
	P "Packed SIMD"
	Q "Quad precision floating point"
	R "RESERVED"
	S "Supervisor mode"
	T "Transactional memory"
	U "User mode"
	V "Vector"
	W "RESERVED"
	X "Non-standard extensions"
	Y "RESERVED"
	Z "RESERVED"
}
