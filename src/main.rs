#![no_std]
#![no_main]
#![feature(allocator_api)]
#![feature(asm)]
#![feature(const_panic)]
#![feature(naked_functions)]
#![feature(once_cell)]
#![feature(nonnull_slice_from_raw_parts)]
#![feature(custom_test_frameworks)]
#![feature(raw)]
#![feature(link_llvm_intrinsics)]
#![test_runner(crate::test::runner)]
#![reexport_test_harness_main = "test_main"]

// TODO read up on the test framework thing. Using macro for now because custom_test_frameworks
// does something stupid complicated with tokenstreams (I just want to log the function name
// damnit)
#[macro_export]
macro_rules! test {
	($name:ident() $code:block) => {
		#[test_case]
		fn $name() {
			log::info(&["  testing ", concat!(module_path!(), "::", stringify!($name))]);
			{ $code }
		}
	};
}


mod alloc;
mod arch;
mod io;
mod log;
mod powerstate;
mod sync;
mod util;

use core::{mem, panic};

#[panic_handler]
fn panic(info: &panic::PanicInfo) -> ! {
	log::fatal(&["Kernel panicked!"]);
	let msg = info.payload();
	if let Some(s) = msg.downcast_ref::<&str>() {
		log::fatal(&["  Message: ", s]);
	//} else if let Some(s) = msg.downcast_ref::<String>() {
	//	log::fatal(&["  Message: ", s.as_str()]);
	}
	if let Some(loc) = info.location() {
		// If more than 8 characters are needed for line/column I'll kill someone
		let mut buf = [0; 8];
		let line = util::usize_to_string(&mut buf, loc.line() as usize, 10, 1).unwrap();
		let mut buf = [0; 8];
		let column = util::usize_to_string(&mut buf, loc.column() as usize, 10, 1).unwrap();
		log::fatal(&["  Source: ", loc.file(), ":", line, ",", column]);
	} else {
		log::fatal(&["  No location info"]);
	}
	let bt_approx = if arch::is_backtrace_accurate() { "" } else { " (approximate)" };
	log::fatal(&["  Backtrace", bt_approx, ":"]);
	arch::backtrace(|sp, fun| {
		const LEN: u8 = 2 * mem::size_of::<*const ()>() as u8;
		let mut buf = [0; LEN as usize];
		let sp = util::usize_to_string(&mut buf, sp as usize, 16, 1).unwrap();
		let mut buf = [0; LEN as usize];
		let fun = util::usize_to_string(&mut buf, fun as usize, 16, 1).unwrap();
		log::fatal(&["    ", sp, ": ", fun]);
	});
	loop {
		powerstate::halt();
	}
}

#[no_mangle]
#[cfg(not(test))]
fn main() {
	use arch::*;
	arch::id().log();
	arch::capabilities().log();
	io::uart::default(|uart| {
		use io::Device;
		let _ = uart.write(b"Greetings!\n");
		let _ = uart.write(b"Type whatever you want, I'll echo it:\n");
		loop {
			let mut buf = [0; 1024];
			let mut i = 0;
			loop {
				match uart.read(&mut buf[i..]) {
					Ok(n) => {
						let _ = uart.write(&buf[i..i + n.get()]);
						i += n.get();
						if buf[i - 1] == b'\n' || buf[i - 1] == b'\r' {
							buf[i - 1] = b'\n';
							break;
						}
					}
					Err(io::ReadError::Empty) => (),
					Err(io::ReadError::BufferIsZeroSized) => break,
					Err(_) => (),
				}
			}
			let _ = uart.write(b"\nYou wrote: ");
			let _ = uart.write(&buf[..i]);
		}
	});
}

#[cfg(test)]
mod test {
	use super::*;

	#[no_mangle]
	#[cfg(test)]
	fn main() {
		test_main();
	}

	pub(super) fn runner(tests: &[&dyn Fn()]) {
		let mut buf = [0; 32];
		let num = util::isize_to_string(&mut buf, tests.len() as isize, 10, 1).unwrap();
		log::info(&[
			"Running ",
			num,
			if tests.len() == 1 { " test" } else { " tests" },
		]);
		for f in tests {
			f();
		}
		log::info(&["Done"]);
	}
}
