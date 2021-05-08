//! Basic logging facilities
//!
//! These are all globally accessible for ease of use

#[derive(PartialEq, PartialOrd)]
#[repr(u8)]
enum LogLevel {
	Fatal = 0,
	Error = 1,
	Warn = 2,
	Info = 3,
	Debug = 4,
}

static mut UART: Option<crate::io::uart::UART> = None;
static LOG_LEVEL: LogLevel = LogLevel::Info;

// FIXME adding cold breaks things because ??????
//#[cold]
fn log(strings: [&str; 3]) {
	// SAFETY: FIXME we should use locks
	unsafe {
		if UART.is_none() {
			UART = Some(crate::io::uart::UART::new());
		}
		let uart = UART.as_mut().unwrap();
		for &s in strings.iter() {
			uart.write_str(s);
		}
	}
}

macro_rules! log {
	($level:ident, $($str:expr)*) => {
		if LOG_LEVEL >= LogLevel::$level {
			log([$($str),*]);
		}
	};
}

pub fn fatal(message: &str) {
	log!(Fatal, "[FATAL] " message "\n");
}

pub fn error(message: &str) {
	log!(Error, "[ERROR] " message "\n");
}

pub fn warn(message: &str) {
	log!(Warn, "[WARN ] " message "\n");
}

pub fn info(message: &str) {
	log!(Info, "[INFO ] " message "\n");
}

pub fn debug(message: &str) {
	log!(Debug, "[DEBUG] " message "\n");
}
