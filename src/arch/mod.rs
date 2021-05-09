#[cfg(any(target_arch = "riscv64"))]
mod riscv;
#[cfg(target_arch = "riscv64")]
pub use riscv::rv64 as riscv64;

use crate::log;

/// A wrappers that allows inspecting the capabilities of the current CPU
#[cfg(target_arch = "riscv64")]
pub struct Capabilities(riscv64::MISA);

impl Capabilities {
	/// Creates a new wrapper around whatever structures that need to be accessed to
	/// get the CPU's capabilities.
	pub fn new() -> Self {
		#[cfg(target_arch = "riscv64")]
		Capabilities(riscv64::MISA::new())
	}

	/// Logs the capabilities of the current CPU
	pub fn log(&self) {
		self.0.log()
	}
}
