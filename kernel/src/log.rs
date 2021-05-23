//! Basic logging facilities
//!
//! These are all globally accessible for ease of use

use crate::io::Device;
use core::fmt;

#[derive(PartialEq, PartialOrd)]
#[repr(u8)]
pub enum LogLevel {
	Fatal = 0,
	Error = 1,
	Warn = 2,
	Info = 3,
	Debug = 4,
}

static LOG_LEVEL: LogLevel = LogLevel::Debug;

#[doc(hidden)]
#[cold]
pub fn log(pre: &str, strings: &[&str]) {
	// TODO how should we handle write failures?
	// Right now UART "can't fail", but what if it
	// does at some point?
	let _ = crate::io::uart::default(|uart| {
		let _ = uart.write_str(pre);
		for &s in strings.iter() {
			let _ = uart.write_str(s);
		}
		let _ = uart.write_str("\n");
	});
}

fn log_prefix(level: LogLevel, prefix: &str, strings: &[&str]) {
	if LOG_LEVEL >= level {
		log(prefix, strings);
	}
}

pub fn fatal(strings: &[&str]) {
	log_prefix(LogLevel::Fatal, "[FATAL] ", strings);
}

pub fn error(strings: &[&str]) {
	log_prefix(LogLevel::Error, "[ERROR] ", strings);
}

pub fn warn(strings: &[&str]) {
	log_prefix(LogLevel::Warn, "[WARN]  ", strings);
}

pub fn info(strings: &[&str]) {
	log_prefix(LogLevel::Info, "[INFO]  ", strings);
}

pub fn debug(strings: &[&str]) {
	log_prefix(LogLevel::Debug, "[DEBUG] ", strings);
}

pub fn debug_str(msg: &str) {
	debug(&[msg]);
}

pub fn debug_usize(msg: &str, num: usize, radix: u8) {
	let mut buf = [0; 128];
	let num = crate::util::usize_to_string(&mut buf, num, radix, 1).unwrap();
	debug(&[msg, " -> ", num]);
}

pub struct Debug(bool);

impl Debug {
	pub fn new() -> Self {
		Self(true)
	}
}

impl fmt::Write for Debug {
	fn write_str(&mut self, string: &str) -> fmt::Result {
		let _ = crate::io::uart::default(|uart| {
			if self.0 {
				uart.write_str("[DEBUG] ");
				self.0 = false;
			}
			uart.write_str(string);
		});
		Ok(())
	}
}

pub macro_rules! debug {
	($($args:expr),* $(,)?) => {{
		use core::fmt::Write;
		writeln!(crate::log::Debug::new(), $($args),*);
	}}
}
