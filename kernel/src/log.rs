//! Basic logging facilities
//!
//! These are all globally accessible for ease of use

use core::fmt;

pub struct Log;

impl fmt::Write for Log {
	fn write_str(&mut self, string: &str) -> fmt::Result {
		for b in string.bytes() {
			crate::arch::riscv::sbi::console_putchar(b);
		}
		Ok(())
	}
}

#[macro_export]
macro_rules! log {
	($($args:tt)*) => {{
		use core::fmt::Write;
		writeln!($crate::log::Log, $($args)*);
	}}
}
