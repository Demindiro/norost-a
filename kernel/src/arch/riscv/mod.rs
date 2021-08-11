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
pub mod sbi;
pub mod vms;

use core::ptr;

/// Structure used to save register state
#[repr(C)]
pub struct RegisterState {
	/// The program counter state.
	pub pc: *const (),
	/// All integer registers except `x0`
	pub x: [usize; 31],
	// /// All FP registers
	//pub f: [usize; 32],
}
impl RegisterState {
	/// Sets the program counter to the given address.
	#[inline(always)]
	pub fn set_pc(&mut self, address: *const ()) {
		self.pc = address;
	}
}
impl Default for RegisterState {
	fn default() -> Self {
		Self {
			x: [0; 31],
			pc: ptr::null(),
			//f: [0; 32],
		}
	}
}

/// Initialize arch-specific structures such as the interrupt table
pub fn init() {
	trap::init();
}

const _: usize = 0 - (4096 - super::Page::SIZE); // Page size check

/// Flags pertaining to ELF files.
///
/// ## References
///
/// [RISC-V ELF psABI specification][elf]
///
/// [elf]: https://github.com/riscv/riscv-elf-psabi-doc/blob/master/riscv-elf.md#elf-object-file
#[allow(dead_code)]
pub mod elf {
	/// The value of the machine byte for this architecture.
	pub const MACHINE: u16 = 0xf3;
	/// Flag indicating whether ELF binaries target the C ABI (i.e. support C extension).
	pub const RVC: u32 = 0x0001;
	/// Flag indicating no support for the floating point ABI.
	pub const FLOAT_ABI_SOFT: u32 = 0x0000;
	/// Flag indicating support for the single precision floating point ABI.
	pub const FLOAT_ABI_SINGLE: u32 = 0x0002;
	/// Flag indicating support for the double precision floating point ABI.
	pub const FLOAT_ABI_DOUBLE: u32 = 0x0004;
	/// Flag indicating support for the quad precision floating point ABI.
	pub const FLOAT_ABI_QUAD: u32 = 0x0006;
	/// Flag indicating whether the E ABI is targeted (i.e. RV32E ISA).
	pub const RVE: u32 = 0x0008;
	/// Flag indicating whether the binary requires RVTSO.
	// TODO brief description of RVTSO
	pub const TSO: u32 = 0x0008;
}

/// Module functions pertaining to setting up traps.
mod trap {

	#[cfg(target_arch = "riscv64")]
	global_asm!("__RISCV64__:");
	#[cfg(target_arch = "riscv32")]
	global_asm!("__RISCV32__:");

	global_asm!(include_str!("types.s"));
	global_asm!(include_str!("registers.s"));
	global_asm!(include_str!("trap.s"));

	/// Initialize the trap CSR and the interrupt table.
	#[inline(always)]
	pub fn init() {
		// SAFETY: the assembly is correct.
		unsafe { trap_init() };
	}

	extern "C" {
		fn trap_init();
	}
}
