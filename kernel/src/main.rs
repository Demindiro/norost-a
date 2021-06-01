#![no_std]
#![no_main]
#![feature(allocator_api)]
#![feature(alloc_layout_extra)]
#![feature(asm)]
#![feature(bindings_after_at)]
#![feature(const_generics)]
#![feature(const_evaluatable_checked)]
#![feature(const_option)]
#![feature(const_panic)]
#![feature(const_raw_ptr_to_usize_cast)]
#![feature(custom_test_frameworks)]
#![feature(destructuring_assignment)]
#![feature(dropck_eyepatch)]
#![feature(inline_const)]
#![feature(global_asm)]
#![feature(lang_items)]
#![feature(linkage)]
#![feature(maybe_uninit_array_assume_init)]
#![feature(maybe_uninit_extra)]
#![feature(maybe_uninit_uninit_array)]
#![feature(naked_functions)]
#![feature(nonnull_slice_from_raw_parts)]
#![feature(option_result_unwrap_unchecked)]
#![feature(optimize_attribute)]
#![feature(once_cell)]
#![feature(panic_info_message)]
#![feature(ptr_metadata)]
#![feature(ptr_internals)]
#![feature(pub_macro_rules)]
#![feature(raw)]
#![feature(slice_ptr_len)]
#![feature(stmt_expr_attributes)]
#![feature(trivial_bounds)]
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
			crate::log::info(&[
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
mod elf;
mod syscall;
mod io;
mod log;
mod memory;
mod powerstate;
mod sync;
mod task;
mod util;

use core::cell::UnsafeCell;
use core::convert::TryInto;
use core::fmt::Write;
use core::num::NonZeroUsize;
use core::{mem, ops, panic, ptr};

/// The default amount of kernel heap memory for the default allocator.
const HEAP_MEM_MAX: usize = 0x100_000;

/// The default MAX_ORDER of the memory manager. This is set to 27 which allows
/// areas up to 512 GiB, or a single "terapage" in RISC-V lingo. This should be
/// sufficient for a very, very long time.
///
/// See [`memory`](crate::memory) for more information.
///
/// ## References
///
/// Mention of "terapages" can be found in [the RISC-V manual for privileged instructions][riscv],
/// "Sv48: Page-Based 48-bit Virtual-Memory System", section 4.5.1, page 37.
///
/// [riscv]: https://github.com/riscv/riscv-isa-manual/releases/download/Ratified-IMFDQC-and-Priv-v1.11/riscv-privileged-20190608.pdf
const MEMORY_MANAGER_MAX_ORDER: usize = 27;

/// A global reference to the memory manager, of which there can only be one managed by the kernel.
pub static MEMORY_MANAGER: BootOnceCell<sync::Mutex<memory::Manager<MEMORY_MANAGER_MAX_ORDER>>> =
	BootOnceCell(UnsafeCell::new(None));

/// A variant of a `OnceCell` that is set during early boot, which means it doesn't check whether
/// the inner value is set. This technically makes it unsafe, but if the invariant is uphold it _is_
/// safe, hence the `deref` method is safe to use.
pub struct BootOnceCell<T>(UnsafeCell<Option<T>>);

impl<T> BootOnceCell<T> {
	/// Sets the inner value. This should be called only once.
	///
	/// ## Safety
	///
	/// This method is called once early at boot time.
	#[inline]
	#[track_caller]
	unsafe fn __init(&self, value: T) {
		debug_assert!(
			self.0.get().as_ref().is_none(),
			"Inner value is already set"
		);
		self.0.get().write(Some(value));
	}
}

impl<T> ops::Deref for BootOnceCell<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		// SAFETY: if the `init` method was called, the inner value is safe to dereference.
		unsafe {
			debug_assert!(self.0.get().as_ref().is_some(), "Inner value isn't set");
			(&*self.0.get()).as_ref().unwrap_unchecked()
		}
	}
}

unsafe impl<T> Sync for BootOnceCell<T> {}

#[panic_handler]
fn panic(info: &panic::PanicInfo) -> ! {
	log::fatal(&["Kernel panicked!"]);
	let msg = info.payload();
	if let Some(msg) = info.message() {
		log::debug!("  Message:  '{}'", msg);
	}
	if let Some(s) = msg.downcast_ref::<&str>() {
		log::fatal(&["  Payload:  ", s]);
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
extern "C" fn main(hart_id: usize, dtb: *const u8, initfs: *const u8, initfs_size: usize) {
	// TODO FIXME
	unsafe {
		// Set pmpcfg0 and pmpaddr0 to allow access to everything in S and U mode
		// Use TOR mode with top address set to -1
		asm!("
			li		t0, -1
			srli	t0, t0, 10
			csrw	0x3b0, t0
			li		t0, 0xf
			csrw	0x3a0, t0
		", lateout("t0") _);
	}


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
						let page_mask = arch::PAGE_SIZE - 1;
						let end = start + size;
						let (start, end) = if (start < kernel_start && end < kernel_start)
							|| (start >= kernel_end && end >= kernel_end)
						{
							// No adjustments needed
							(start, end)
						} else if start >= kernel_start && end >= kernel_end {
							// Adjust upwards & align to page boundary
							let delta = kernel_end - start;
							let start = (start + delta + page_mask) & !page_mask;
							(start, end)
						} else {
							// While other layouts are technically possible, I assume it's uncommon
							// because why would anyone make it harder than necessary?
							unimplemented!();
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

	// Initialize the memory manager
	let (address, size) = heap.expect("No memory device (check the DTB!)");
	// SAFETY: The DTB told us this address range is valid. We also ensured no existing memory will
	// be overwritten.
	let mm = NonZeroUsize::new(size / arch::PAGE_SIZE).expect("Memory range is zero-sized");
	let mm = unsafe { memory::Manager::<MEMORY_MANAGER_MAX_ORDER>::new(address.cast(), mm) };
	// SAFETY: the init function hasn't been called yet.
	unsafe {
		MEMORY_MANAGER.__init(sync::Mutex::new(mm));
	}

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
	log::info(&["Useable memory range: ", start, " - ", end]);

	// Initialize trap table now that the most important setup is done.
	arch::init();

	log::debug!("initfs: {:p}, {}", initfs, initfs_size);

	// Run init
	// 128 KiB with 4 KiB pages
	let alloc = MEMORY_MANAGER.lock().allocate(5).expect("Failed to alloc initfs heap");
	// SAFETY: the memory is valid and not in use by anything else.
	let alloc = unsafe {
		alloc::allocators::WaterMark::new(alloc.start().cast(), arch::PAGE_SIZE << alloc.order())
	};
	// SAFETY: a valid init pointer and size should have been passed by boot.s.
	let init = unsafe { core::slice::from_raw_parts(initfs, initfs_size) };
	let init = elf::ELF::parse(init.as_ref(), &alloc).expect("Invalid ELF file");
	let init = init.create_task().expect("Failed to create init task");
	init.next();
}

#[cfg(test)]
mod test {
	use super::*;
	use core::num::NonZeroUsize;
	use core::ptr::NonNull;

	#[no_mangle]
	#[cfg(test)]
	fn main() {
		test_main();
	}

	const MEMORY_MANAGER_ADDRESS: NonNull<arch::Page> =
		unsafe { NonNull::new_unchecked(0x8100_0000 as *mut _) };

	pub(super) fn runner(tests: &[&dyn Fn()]) {
		let mut buf = [0; 32];
		let num = util::isize_to_string(&mut buf, tests.len() as isize, 10, 1).unwrap();
		log::info(&[
			"Running ",
			num,
			if tests.len() == 1 { " test" } else { " tests" },
		]);
		arch::init();
		for f in tests {
			// Reinitialize the memory manager each time in case of leaks or something else
			// overwriting it's state.
			// Incredibly unsafe, but w/e.
			unsafe {
				MEMORY_MANAGER
					.0
					.get()
					.write(Some(sync::Mutex::new(memory::Manager::new(
						MEMORY_MANAGER_ADDRESS,
						NonZeroUsize::new(256).unwrap(),
					))));
			}
			f();
		}
		log::info(&["Done"]);
	}
}
