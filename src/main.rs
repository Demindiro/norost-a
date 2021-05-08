#![no_std]
#![no_main]

mod io;

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
	loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
	// SAFETY: we call UART::new only once
	let mut uart = unsafe { io::uart::UART::new() };
	uart.write_str("Hello, world!\n");
	loop {}
}
