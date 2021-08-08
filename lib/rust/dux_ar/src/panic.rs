//! Panic handler stub for static libs.

use kernel::sys_log;

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
	sys_log!("Panic!");
	if let Some(m) = info.message() {
		sys_log!("  Message: {}", m);
	}
	if let Some(l) = info.location() {
		sys_log!("  Location: {}", l);
	}
	loop {}
}
