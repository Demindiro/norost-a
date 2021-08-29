#![no_std]
#![no_main]
#![feature(allocator_api)]
#![feature(alloc_layout_extra)]
#![feature(arbitrary_enum_discriminant)]
#![feature(asm)]
#![feature(const_mut_refs)]
#![feature(const_option)]
#![feature(const_panic)]
#![feature(const_ptr_is_null)]
#![feature(const_ptr_offset)]
#![feature(const_raw_ptr_deref)]
#![feature(destructuring_assignment)]
#![feature(dropck_eyepatch)]
#![feature(global_asm)]
#![feature(lang_items)]
#![feature(linkage)]
#![feature(maybe_uninit_array_assume_init)]
#![feature(maybe_uninit_extra)]
#![feature(maybe_uninit_uninit_array)]
#![feature(naked_functions)]
#![feature(never_type)]
#![feature(nonnull_slice_from_raw_parts)]
#![feature(option_result_unwrap_unchecked)]
#![feature(optimize_attribute)]
#![feature(once_cell)]
#![feature(panic_info_message)]
#![feature(ptr_metadata)]
#![feature(ptr_internals)]
#![feature(slice_ptr_len)]
#![feature(stmt_expr_attributes)]
#![feature(trivial_bounds)]
#![feature(untagged_unions)]
#![feature(link_llvm_intrinsics)]

#[macro_use]
mod log;
#[macro_use]
mod util;

mod allocator;
mod arch;
mod driver;
mod elf;
mod memory;
mod powerstate;
mod sync;
mod syscall;
mod task;

use core::convert::TryInto;
use core::{mem, panic, ptr};
use util::OnceCell;

static PLATFORM_INFO_SIZE: OnceCell<usize> = OnceCell::new(0);
static PLATFORM_INFO_PHYS_PTR: OnceCell<usize> = OnceCell::new(0);

#[panic_handler]
fn panic(info: &panic::PanicInfo) -> ! {
	log!("Kernel panicked!");
	if let Some(msg) = info.message() {
		log!("  Message:  {:?}", msg);
	}
	if let Some(loc) = info.location() {
		log!("  Source:   {}:{},{}", loc.file(), loc.line(), loc.column());
	} else {
		log!("  No location info");
	}
	let bt_approx = if arch::is_backtrace_accurate() {
		""
	} else {
		" (approximate)"
	};
	log!("  Backtrace{}:", bt_approx);
	arch::backtrace(|sp, fun| log!("    {:p}: {:p}", sp, fun));
	loop {
		powerstate::halt();
	}
}

#[cfg(feature = "dump-dtb")]
fn dump_dtb(dtb: &driver::DeviceTree) {
	log!("Device tree:");
	log!("  Reserved memory regions:");
	for rmr in dtb.reserved_memory_regions() {
		let addr = rmr.address.get() as usize;
		let size = rmr.size.get() as usize;
		log!("  {:x} <-> {:x} (len: {:x})", addr, addr + size, size);
	}

	fn print_node(level: usize, mut node: driver::Node) {
		log!("{0:>1$}{2} {{", "", level * 2, node.name);
		while let Some(property) = node.next_property() {
			if property.value.len() > 0
				&& property.value[..property.value.len() - 1]
					.iter()
					// Everything between ' ' and '~' is human-readable
					.all(|&c| b' ' <= c && c <= b'~')
				&& property.value.last().unwrap() == &0
			{
				// SAFETY: The string is a valid null-terminated string
				let s = unsafe {
					core::str::from_utf8_unchecked(&property.value[..property.value.len() - 1])
				};
				log!("{0:>1$}{2} = {3:?}", "", level * 2 + 2, property.name, s);
			} else {
				log!(
					"{0:>1$}{2} = {3:02x?}",
					"",
					level * 2 + 2,
					property.name,
					&property.value
				);
			}
		}
		while let Some(node) = node.next_child_node() {
			print_node(level + 1, node);
		}
		log!("{0:>1$}}}", "", level * 2);
	}
	let mut interpreter = dtb.interpreter();
	while let Some(mut node) = interpreter.next_node() {
		print_node(1, node);
	}
}

#[no_mangle]
#[cfg(not(test))]
extern "C" fn main(
	hart_id: usize,
	dtb_ptr: *const arch::Page,
	_kernel: *const u8,
	_kernel_size: usize,
	init: *const u8,
	init_size: usize,
) {
	// Initialize trap table immediately so we can catch errors as early as possible.
	arch::init();

	// Parse DTB and reserve some memory for heap usage
	let dtb = unsafe { driver::DeviceTree::parse_dtb(dtb_ptr.cast()).unwrap() };
	#[cfg(feature = "dump-dtb")]
	dump_dtb(&dtb);

	let mut interpreter = dtb.interpreter();
	let mut root = interpreter.next_node().expect("No root node");

	let mut address_cells = None;
	let mut size_cells = None;
	let mut model = "";
	let mut boot_args = "";
	let mut stdout = "";

	let log_err_malformed_prop = |name| log!("Value of '{}' is malformed", name);

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
	let mut reserved_memory = [(0, 0); 16];

	// TODO see comment at reserved_memory_regions function.
	dtb.reserved_memory_regions().for_each(|_| ());

	while let Some(mut node) = root.next_child_node() {
		if node.name.starts_with("reserved-memory") {
			// Also, what is the significance of the "ranges" property? It's
			// always empty anyways.
			// Ref: https://elixir.bootlin.com/linux/latest/source/Documentation/devicetree/bindings/reserved-memory/reserved-memory.txt
			let mut address_cells = address_cells;
			let mut size_cells = size_cells;
			while let Some(prop) = node.next_property() {
				match prop.name {
					"#address-cells" => {
						address_cells = u32::from_be_bytes(prop.value.try_into().unwrap())
					}
					"#size-cells" => {
						size_cells = u32::from_be_bytes(prop.value.try_into().unwrap())
					}
					"ranges" => (),
					_ => (),
				}
			}
			let mut rm_i = 0;
			while let Some(mut child) = node.next_child_node() {
				while let Some(prop) = child.next_property() {
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
							log!("{:?}", prop.value);
							log!("0x{:x} - 0x{:x}", start, start + size - 1);
							reserved_memory[rm_i] = (start, start + size - 1);
							rm_i += 1;
						}
						_ => (),
					}
				}
			}
		} else if heap.is_none() && node.name.starts_with("memory@") {
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
						let page_mask = arch::Page::SIZE - 1;
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
	mem::drop(root);
	interpreter.finish();

	memory::reserved::dump_vms_map();

	// Initialize the memory manager
	//let (address, size) = heap.expect("No memory device (check the DTB!)");
	// FIXME this is utter shit
	let (address, size) = (0x8400_0000, 2048 * arch::Page::SIZE);
	// SAFETY: The DTB told us this address range is valid. We also ensured no existing memory will
	// be overwritten.
	let mm = unsafe {
		memory::PPNRange::from_ptr(address, (size / arch::Page::SIZE).try_into().unwrap())
	};
	unsafe { memory::mem_add_ranges(&mut [mm]) };

	// Initialize the device list
	struct IterProp<'a> {
		properties: [Option<(&'a str, &'a [u32])>; 16],
		counter: usize,
	}

	impl<'a> Iterator for IterProp<'a> {
		type Item = (&'a str, &'a [u32]);

		fn next(&mut self) -> Option<Self::Item> {
			self.counter += 1;
			self.properties[self.counter - 1]
		}
	}

	// Remap FDT to kernel global space
	unsafe {
		PLATFORM_INFO_PHYS_PTR.set(dtb_ptr as usize);
	}
	unsafe {
		PLATFORM_INFO_SIZE.set((dtb.total_size() + arch::Page::SIZE - 1) / arch::Page::SIZE);
	}
	let mut addr = memory::reserved::DEVICE_TREE.start;
	for i in 0..(dtb.total_size() + arch::Page::SIZE - 1) / arch::Page::SIZE {
		unsafe {
			let p = arch::Map::Private(memory::PPN::from_ptr(dtb_ptr.add(i) as usize));
			arch::VMS::add(
				addr,
				p,
				arch::vms::RWX::R,
				arch::vms::Accessibility::KernelGlobal,
			)
			.unwrap();
			addr = addr.next().unwrap();
		}
	}

	// Get init segments
	#[rustfmt::ignore]
	let mut segments = [
		None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
		None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
		None, None,
	];
	let mut entry = core::ptr::null();
	// SAFETY: a valid init pointer and size should have been passed by boot.s.
	let init = unsafe { core::slice::from_raw_parts(init, init_size) };
	elf::parse(init.as_ref(), &mut segments[..], &mut entry);

	use arch::vms::VirtualMemorySystem;

	// Unmap identity maps (making the init elf file inaccessible)
	arch::VMS::clear_identity_maps();

	// Create init task and map pages.
	let init = task::Task::new(arch::VMS::current()).expect("Failed to create task");
	for s in segments.iter_mut().filter_map(|s| s.as_mut()) {
		let mut a = s.address;
		while let Some(ppn) = s.ppn.pop_base() {
			let ppn = arch::Map::Private(ppn);
			arch::VMS::add(
				a,
				ppn,
				/* s.flags */ arch::vms::RWX::RWX,
				arch::vms::Accessibility::UserLocal,
			)
			.unwrap();
			a = a.next().unwrap();
		}

		arch::set_supervisor_userpage_access(true);
		for i in s.clear_from..s.clear_to {
			unsafe { ptr::write_volatile(s.address.as_ptr().cast::<u8>().add(i), 0) };
		}
		arch::set_supervisor_userpage_access(false);
	}
	init.set_pc(entry);

	task::Group::new(init).expect("failed to create init task group");

	let _ = (boot_args, stdout, model);

	arch::enable_interrupts(true);
	task::Executor::init(hart_id.try_into().expect("hart id higher than supported"));
	task::Executor::next();
}
