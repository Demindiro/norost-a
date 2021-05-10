#![no_std]
#![no_main]
#![feature(allocator_api)]
#![feature(asm)]
#![feature(const_panic)]
#![feature(custom_test_frameworks)]
#![feature(naked_functions)]
#![feature(nonnull_slice_from_raw_parts)]
#![feature(once_cell)]
#![feature(panic_info_message)]
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
			log::info(&[
				"  testing ",
				concat!(module_path!(), "::", stringify!($name)),
			]);
			{
				$code
			}
		}
	};
}

mod alloc;
mod arch;
mod driver;
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
	if let Some(msg) = info.message() {
		if let Some(msg) = msg.as_str() {
			log::fatal(&["  Message:  '", msg, "'"]);
		}
	}
	if let Some(s) = msg.downcast_ref::<&str>() {
		log::fatal(&["  Payload:  ", s]);
		//} else if let Some(s) = msg.downcast_ref::<String>() {
		//	log::fatal(&["  Message: ", s.as_str()]);
	}
	if let Some(loc) = info.location() {
		// If more than 8 characters are needed for line/column I'll kill someone
		let mut buf = [0; 8];
		let line = util::usize_to_string(&mut buf, loc.line() as usize, 10, 1).unwrap();
		let mut buf = [0; 8];
		let column = util::usize_to_string(&mut buf, loc.column() as usize, 10, 1).unwrap();
		log::fatal(&["  Source:   ", loc.file(), ":", line, ",", column]);
	} else {
		log::fatal(&["  No location info"]);
	}
	let bt_approx = if arch::is_backtrace_accurate() {
		""
	} else {
		" (approximate)"
	};
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

#[cfg(feature = "dump-dtb")]
fn dump_dtb(dtb: &driver::DeviceTree) {
	log::debug(&["Device tree:"]);
	let mut buf = [0; 32];
	let num = util::usize_to_string(&mut buf, dtb.boot_cpu_id() as usize, 16, 1).unwrap();
	log::debug(&["  Boot CPU physical ID: ", num]);
	log::debug(&["  Reserved memory regions:"]);
	for rmr in dtb.reserved_memory_regions() {
		let mut buf = [0; 32];
		let addr = util::usize_to_string(&mut buf, rmr.address.get() as usize, 16, 1).unwrap();
		let mut buf = [0; 32];
		let size = util::usize_to_string(&mut buf, rmr.size.get() as usize, 16, 1).unwrap();
		log::debug(&["    ", addr, " ", size]);
	}
	fn print_node(level: usize, buf: &mut [&str; 64], mut node: driver::Node) {
		buf[level * 2] = "'";
		buf[level * 2 + 1] = node.name;
		buf[level * 2 + 2] = "'";
		log::debug(&buf[..level * 2 + 3]);
		buf[level * 2] = "  Properties:";
		log::debug(&buf[..level * 2 + 1]);
		while let Some(property) = node.next_property() {
			buf[level * 2] = "    '";
			buf[level * 2 + 1] = property.name;
			buf[level * 2 + 2] = "'";
			log::debug(&buf[..level * 2 + 3]);
		}
		buf[level * 2] = "  Child nodes:";
		log::debug(&buf[..level * 2 + 1]);
		while let Some(node) = node.next_child_node() {
			buf[level * 2] = "  ";
			buf[level * 2 + 1] = "  ";
			print_node(level + 1, buf, node);
		}
	}
	log::debug(&["  Nodes:"]);
	let mut interpreter = dtb.interpreter();
	let mut buf = ["  "; 64];
	while let Some(mut node) = interpreter.next_node() {
		print_node(1, &mut buf, node);
	}
}

#[no_mangle]
#[cfg(not(test))]
fn main(hart_id: usize, dtb: *const u8) {
	use arch::*;
	arch::id().log();
	arch::capabilities().log();

	let dtb = unsafe { driver::DeviceTree::parse_dtb(dtb).unwrap() };
	#[cfg(feature = "dump-dtb")]
	dump_dtb(&dtb);

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
