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
pub mod vms;
pub mod sbi;

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

use core::{mem, ptr};

/// Initialize arch-specific structures such as the interrupt table
pub fn init() {
	trap::init();
}


/// A representation of a single memory page.
// TODO figure out how to set repr align based on a constant
#[repr(align(4096))]
pub struct Page {
	data: [[usize; 8]; PAGE_SIZE / mem::size_of::<[usize; 8]>()],
}

const _PAGE_SIZE_CHECK: usize = 0 - (4096 - mem::size_of::<Page>());

impl Page {
	/// Overwrite this page with zeroes
	#[inline(always)]
	#[optimize(speed)]
	#[allow(dead_code)]
	pub fn clear(&mut self) {
		for e in self.data.iter_mut() {
			// Manual loop unrolling because the compiler is a dumb brick.
			unsafe {
				ptr::write_volatile(&mut e[0], 0);
				ptr::write_volatile(&mut e[1], 0);
				ptr::write_volatile(&mut e[2], 0);
				ptr::write_volatile(&mut e[3], 0);
				ptr::write_volatile(&mut e[4], 0);
				ptr::write_volatile(&mut e[5], 0);
				ptr::write_volatile(&mut e[6], 0);
				ptr::write_volatile(&mut e[7], 0);
			}
		}
	}
}

/// The size of a single memory page, which is 4KB for all RISC-V architectures.
pub const PAGE_SIZE: usize = 4096;

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
