#![no_std]
#![no_main]
#![feature(allocator_api)]
#![feature(alloc_layout_extra)]
#![feature(asm)]
#![feature(const_panic)]
#![feature(custom_test_frameworks)]
#![feature(dropck_eyepatch)]
#![feature(lang_items)]
#![feature(maybe_uninit_extra)]
#![feature(naked_functions)]
#![feature(nonnull_slice_from_raw_parts)]
#![feature(option_result_unwrap_unchecked)]
#![feature(once_cell)]
#![feature(panic_info_message)]
#![feature(ptr_metadata)]
#![feature(ptr_internals)]
#![feature(raw)]
#![feature(slice_ptr_len)]
#![feature(link_llvm_intrinsics)]
#![test_runner(crate::test::runner)]
#![reexport_test_harness_main = "test_main"]

/// The default amount of kernel heap memory for the default allocator.
// 1 MiB should be plenty for now and probably forever
const HEAP_MEM_MAX: usize = 0x100_000;

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
mod vfs;

use core::convert::TryInto;
use core::{mem, panic, ptr};

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
	// Log architecture info
	use arch::*;
	arch::id().log();
	arch::capabilities().log();

	// Parse DTB and reserve some memory for heap usage
	let dtb = unsafe { driver::DeviceTree::parse_dtb(dtb).unwrap() };
	#[cfg(feature = "dump-dtb")]
	dump_dtb(&dtb);

	let mut interpreter = dtb.interpreter();
	let mut root = interpreter.next_node().expect("No root node");

	let mut address_cells = None;
	let mut size_cells = None;
	let mut model = "";
	let mut boot_args = "";
	let mut stdout = "";

	let log_err_malformed_prop = |name| log::error(&["Value of '", name, "' is malformed"]);

	while let Some(prop) = root.next_property() {
		match prop.name {
			"#address-cells" => {
				let num = prop.value.try_into().expect("Malformed #address-cells");
				address_cells = Some(u32::from_be_bytes(num));
			}
			"#size-cells" => {
				let num = prop.value.try_into().expect("Malformed #size-cells");
				size_cells = Some(u32::from_be_bytes(num));
			}
			"model" => {
				if let Ok(m) = core::str::from_utf8(prop.value) {
					model = m;
				} else {
					log_err_malformed_prop("model");
				}
			}
			// Ignore other properties
			_ => (),
		}
	}

	let address_cells = address_cells.expect("Address cells isn't set");
	let size_cells = size_cells.expect("Address cells isn't set");

	let mut heap = None;

	while let Some(mut node) = root.next_child_node() {
		if heap.is_none() && node.name.starts_with("memory@") {
			while let Some(prop) = node.next_property() {
				match prop.name {
					"reg" => {
						let val = prop.value;
						let (start, val) = match address_cells {
							0 => (0, val),
							1 => (
								u32::from_be_bytes(val[..4].try_into().unwrap()) as usize,
								&val[4..],
							),
							2 => (
								u64::from_be_bytes(val[..8].try_into().unwrap()) as usize,
								&val[8..],
							),
							_ => panic!("Unsupported address size"),
						};
						let size = match size_cells {
							0 => 0,
							1 => u32::from_be_bytes(val.try_into().unwrap()) as usize,
							2 => u64::from_be_bytes(val.try_into().unwrap()) as usize,
							_ => panic!("Unsupported size size"),
						};
						// Ensure we don't ever allocate 0x0.
						let start = start
							+ if start == 0 {
								mem::size_of::<usize>()
							} else {
								0
							};
						let (kernel_start, kernel_end): (usize, usize);
						// SAFETY: loading symbols is safe
						unsafe {
							asm!("
								la	t0, _kernel_start
								la	t1, _kernel_end
								", out("t0") kernel_start, out("t1") kernel_end
							);
						}
						let max_end = start + size;
						let end = start + HEAP_MEM_MAX;
						let (start, end) = if (start < kernel_start && end < kernel_start)
							|| (start >= kernel_end && end >= kernel_end)
						{
							// No adjustments needed
							(start, end)
						} else if start >= kernel_start && end >= kernel_end {
							// Adjust upwards
							let delta = kernel_end - start;
							let start = start + delta;
							let end = end + delta;
							if end > max_end {
								(start, max_end)
							} else {
								(start, end)
							}
						} else {
							// While other layouts are technically possible, I assume it's uncommon
							// because why would anyone make it harder than necessary?
							todo!();
						};
						heap = Some((ptr::NonNull::new(start as *mut u8).unwrap(), end - start));
					}
					_ => (),
				}
			}
		} else if node.name.starts_with("chosen") {
			while let Some(prop) = node.next_property() {
				if let Ok(value) = core::str::from_utf8(prop.value) {
					match prop.name {
						"bootargs" => boot_args = value,
						"stdout-path" => stdout = value,
						_ => (),
					}
				} else {
					log_err_malformed_prop(prop.name);
				}
			}
		}
	}

	// Allocate a heap
	let (address, size) = heap.expect("No address for heap allocation");
	// SAFETY: The DTB told us this address is valid. We also ensured no existing
	// memory will be overwritten.
	let heap = unsafe { alloc::allocators::WaterMark::new(address, size) };

	// Log some of the properties we just fetched
	log::info(&["Device model: '", model, "'"]);
	log::info(&["Boot arguments: '", boot_args, "'"]);
	log::info(&["Dumping logs on '", stdout, "'"]);

	const DIGITS: u8 = 2 * mem::size_of::<usize>() as u8;
	let start = address.as_ptr() as usize;
	let end = start + size;
	let mut buf = [0; DIGITS as usize];
	let start = util::usize_to_string(&mut buf, start, 16, DIGITS).unwrap();
	let mut buf = [0; DIGITS as usize];
	let end = util::usize_to_string(&mut buf, end, 16, DIGITS).unwrap();
	log::info(&["Kernel heap: ", start, " - ", end]);

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
