//! Basic logging facilities
//!
//! These are all globally accessible for ease of use

use crate::io::Device;

#[derive(PartialEq, PartialOrd)]
#[repr(u8)]
pub enum LogLevel {
	Fatal = 0,
	Error = 1,
	Warn = 2,
	Info = 3,
	Debug = 4,
}

static mut UART: Option<crate::io::uart::UART> = None;
static LOG_LEVEL: LogLevel = LogLevel::Info;

#[doc(hidden)]
#[cold]
pub fn log(pre: &str, strings: &[&str]) {
	// SAFETY: FIXME we should use locks
	unsafe {
		if UART.is_none() {
			UART = Some(crate::io::uart::UART::new());
		}
		let uart = UART.as_mut().unwrap();
		// TODO how should we handle write failures?
		// Right now UART "can't fail", but what if it
		// does at some point?
		let _ = uart.write_str(pre);
		for &s in strings.iter() {
			let _ = uart.write_str(s);
		}
		let _ = uart.write_str("\n");
	}
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
	log_prefix(LogLevel::Warn, "[WARN ] ", strings);
}

pub fn info(strings: &[&str]) {
	log_prefix(LogLevel::Info, "[INFO ] ", strings);
}

pub fn debug(strings: &[&str]) {
	log_prefix(LogLevel::Debug, "[DEBUG] ", strings);
}
