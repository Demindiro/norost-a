#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![feature(asm)]
#![test_runner(crate::test::runner)]

mod io;
mod log;
mod test;
mod idle;

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
	loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
	log::info("Hello, world!");
	log::warn("This is bullshit");
	loop {
		idle::halt();
	}
}
