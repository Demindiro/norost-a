#![no_std]
#![no_main]
#![feature(allocator_api)]
#![feature(asm)]
#![feature(naked_functions)]
#![feature(once_cell)]
#![feature(nonnull_slice_from_raw_parts)]
#![feature(custom_test_frameworks)]
#![feature(raw)]
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

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
	loop {}
}

#[no_mangle]
#[cfg(not(test))]
fn main() {
	arch::Capabilities::new().log();
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
