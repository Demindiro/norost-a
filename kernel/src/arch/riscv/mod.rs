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

/// The size of a single memory page, which is 4KB for all RISC-V architectures.
pub const PAGE_SIZE: usize = 4096;

/// A wrapper around the contents of the `misa` register, which describes the base ISA and it's
/// supported extensions.
pub struct MISA;

/// A wrapper to query vendor information
pub struct ID;

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

/// A listing of all CSRs
mod csr {
	macro_rules! csr_table {
		{
			$($value:literal $name:ident)*
		} => {
			$(#[allow(dead_code)] pub const $name: u16 = $value;)*
		};
		($base_value:literal $name:ident $start:literal $end:literal) => {
			#[allow(dead_code)]
			#[allow(unused_comparisons)]
			pub const fn $name(index: u16) -> u16 {
				assert!($start <= index, "Index is not within range");
				assert!(index <= $end, "Index is not within range");
				$base_value + index
			}
		};
	}

	csr_table! {
		0x000 USTATUS
		0x004 UIE
		0x005 UTVEC

		0x040 USCRATCH
		0x041 UEPC
		0x042 UCAUSE
		0x043 UTVAL
		0x044 UIP

		0x001 FFLAGS
		0x002 FRM
		0x003 FCSR

		0xc00 CYCLE
		0xc01 TIME
		0xc02 INSTRET

		0xc80 CYCLEH
		0xc81 TIMEH
		0xc82 INSTRETH

		0x100 SSTATUS
		0x102 SEDELEG
		0x103 SIDELEG
		0x104 SIE
		0x105 STVEC
		0x106 SCOUNTEREN

		0x140 SSCRATCH
		0x141 SEPC
		0x142 SCAUSE
		0x143 STVAL
		0x144 SIP

		0x180 SATP

		0xf11 MVENDORID
		0xf12 MARCHID
		0xf13 MIMP
		0xf14 MHARTID

		0x300 MSTATUS
		0x301 MISA
		0x302 MEDELEG
		0x303 MIDELEG
		0x304 MIE
		0x305 MTVEC
		0x306 MCOUNTEREN

		0x340 MSCRATCH
		0x341 MEPC
		0x342 MCAUSE
		0x343 MTVAL
		0x344 MIP

		0xb00 MCYCLE
		0xb02 MINSTRET

		0xb80 MCYCLEH
		0xb02 MINSTRETH

		0x320 MCOUNTINHIBIT

		0x7a0 TSELECT

		0x7b0 DCSR
		0x7b1 DPC
	}

	csr_table!(0xc00 hpmcounter 3 31);
	csr_table!(0xc80 hpmcounterh 3 31);
	csr_table!(0x3a0 pmpcfg 0 3);
	csr_table!(0x3a0 pmpaddr 0 15);
	csr_table!(0xb00 mhpmcounter 3 31);
	csr_table!(0xb80 mhpmcounterh 3 31);
	csr_table!(0x320 mhpmevent 3 31);
	csr_table!(0x7a0 tdata 1 3);
	csr_table!(0x7b2 dscratch 0 1);
}

macro_rules! csr {
	($name:ident) => {{
		let t: usize;
		asm!("csrrs a0, {csr}, zero", csr = const csr::$name, out("a0") t);
		t
	}};
	($name:ident($index:literal)) => {{
		const CSR: usize = csr::$name($index);
		let t: usize;
		asm!("csrrs a0, {csr}, zero", csr = const CSR, out("a0") t);
		t
	}};
}

impl MISA {
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
				csr = const csr::MISA,
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
		// SAFETY: Inspecting `misa` is always safe.
		unsafe { csr!(MISA) & (1 << extension as u8) != 0 }
	}
}

impl ID {}

impl MXL {
	/// Returns the MXL as a human-readable string
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::XLEN32 => "32",
			Self::XLEN64 => "64",
			Self::XLEN128 => "128",
		}
	}

	/// Returns the XLEN of the MXL as an `u8`
	pub fn xlen(&self) -> u8 {
		match self {
			Self::XLEN32 => 32,
			Self::XLEN64 => 64,
			Self::XLEN128 => 128,
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

impl super::Capabilities for MISA {
	/// Wraps the contents of the `misa` register. This constructor is a NOOP, though
	/// this may change.
	#[cold]
	fn new() -> Self {
		Self
	}

	/// Logs the value of `misa` in human-readable format
	#[cold]
	fn log(&self) {
		// log(1 << 128) / log(16) == 32
		let mut buf = [0; 32];
		// SAFETY: Inspecting `misa` is always safe
		let misa = unsafe { csr!(MISA) };
		let mxl = self.mxl();
		let misa = util::usize_to_string(&mut buf, misa, 16, mxl.xlen() / 4).unwrap();
		log::info(&["MISA = ", misa]);
		log::info(&["  MXL = ", mxl.as_str()]);
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

impl super::ID for ID {
	#[cold]
	fn new() -> Self {
		Self
	}

	#[cold]
	fn jedec(&self) -> super::JEDEC {
		// SAFETY: accessing the mvendorid CSR is safe.
		let jedec = unsafe { csr!(MVENDORID) };
		super::JEDEC {
			stop: (jedec & (1 << 6)) as u8,
			continuation: jedec >> 6,
		}
	}

	#[cold]
	fn arch(&self) -> usize {
		// SAFETY: accessing the marchid is safe.
		unsafe { csr!(MARCHID) }
	}

	#[cold]
	fn hart(&self) -> usize {
		// SAFETY: accessing the hartid is safe.
		unsafe { csr!(MHARTID) }
	}
}
