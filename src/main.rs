#![no_std]
#![no_main]

#![feature(asm)]
#![feature(naked_functions)]

#![feature(custom_test_frameworks)]
#![test_runner(crate::test::runner)]
#![reexport_test_harness_main = "test_main"]

mod io;
mod log;
mod idle;
mod util;

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
	loop {}
}

#[no_mangle]
#[cfg(not(test))]
fn main() {
	log::info("Hello, world!");
	log::warn("This is bullshit");
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
		let num = util::isize_to_string(&mut buf, tests.len() as isize).unwrap();
		log::info("Running tests");
		log::info(num);
		for f in tests {
			f();
		}
		log::info("Done");
	}

	#[test_case]
	fn wakawaka() {
		log::fatal("WAKA WAKA EE EE");
	}
}
