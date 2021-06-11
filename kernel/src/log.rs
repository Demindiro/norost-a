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
		writeln!($crate::log::Log, $($args)*).unwrap();
	}}
}

// Shamelessly copied from stdlib.
#[macro_export]
macro_rules! dbg {
    () => {
        $crate::log!("[{}:{}]", file!(), line!());
    };
    ($val:expr $(,)?) => {
        // Use of `match` here is intentional because it affects the lifetimes
        // of temporaries - https://stackoverflow.com/a/48732525/1063961
        match $val {
            tmp => {
                $crate::log!("[{}:{}] {} = {:#?}",
                    file!(), line!(), stringify!($val), &tmp);
                tmp
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        ($($crate::dbg!($val)),+,)
    };
}
