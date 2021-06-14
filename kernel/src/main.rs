#![no_std]
#![no_main]
#![feature(allocator_api)]
#![feature(alloc_layout_extra)]
#![feature(asm)]
#![feature(bindings_after_at)]
#![feature(const_option)]
#![feature(const_panic)]
#![feature(const_raw_ptr_to_usize_cast)]
#![feature(custom_test_frameworks)]
#![feature(destructuring_assignment)]
#![feature(dropck_eyepatch)]
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
#![feature(untagged_unions)]
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
			log!(concat!("  testing ", module_path!(), "::", stringify!($name)));
			{
				$code
			}
		}
	};
}

#[macro_use]
mod log;

mod arch;
mod driver;
mod elf;
mod syscall;
mod memory;
mod powerstate;
mod sync;
mod task;
mod util;

use core::convert::TryInto;
use core::{mem, panic, ptr};

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
			if property.value.len() > 0 &&
				property.value[..property.value.len() - 1]
					.iter()
					// Everything between ' ' and '~' is human-readable
					.all(|&c| b' ' <= c && c <= b'~') &&
				property.value.last().unwrap() == &0
			{
				// SAFETY: The string is a valid null-terminated string
				let s = unsafe {
					core::str::from_utf8_unchecked(&property.value[..property.value.len() - 1])
				};
				log!("{0:>1$}{2} = {3:?}", "", level * 2 + 2, property.name, s);
			} else {
				log!("{0:>1$}{2} = {3:02x?}", "", level * 2 + 2, property.name, &property.value);
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
extern "C" fn main(_hart_id: usize, dtb: *const u8, initfs: *const u8, initfs_size: usize) {

	// Initialize trap table immediately so we can catch errors as early as possible.
	arch::init();

	/*
	// Log architecture info
	use arch::*;
	arch::id().log();
	arch::capabilities().log();
	*/

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
		if node.name.starts_with("reserved-memory")  {
			// Also, what is the significance of the "ranges" property? It's
			// always empty anyways.
			// Ref: https://elixir.bootlin.com/linux/latest/source/Documentation/devicetree/bindings/reserved-memory/reserved-memory.txt
			let mut address_cells = address_cells;
			let mut size_cells = size_cells;
			while let Some(prop) = node.next_property() {
				match prop.name {
					"#address-cells" => address_cells = u32::from_be_bytes(prop.value.try_into().unwrap()),
					"#size-cells" => size_cells = u32::from_be_bytes(prop.value.try_into().unwrap()),
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
	mem::drop(root);
	mem::drop(interpreter);

	memory::reserved::dump_vms_map();

	// Initialize the memory manager
	//let (address, size) = heap.expect("No memory device (check the DTB!)");
	// FIXME this is utter shit
	let (address, size) = (0x8400_0000, 256 * arch::PAGE_SIZE);
	// SAFETY: The DTB told us this address range is valid. We also ensured no existing memory will
	// be overwritten.
	let mm = unsafe { memory::PPNRange::from_ptr(address, (size / arch::PAGE_SIZE).try_into().unwrap()) };
	unsafe { memory::mem_add_ranges(&mut [mm]) };

	// Log some of the properties we just fetched
	log!("Device model: '{}'", model);
	log!("Boot arguments: '{}'", boot_args);
	log!("Dumping logs on '{}'", stdout);

	let start = address;
	let end = start + size;
	log!("Useable memory range: 0x{:x}-0x{:x}", start, end);

	log!("initfs: {:p}, {}", initfs, initfs_size);


	// Testing some stuff, hang on.
	unsafe {
		#[repr(C)]
		#[derive(Debug)]
		struct PCIHeader {
			vendor_id: u16,
			device_id: u16,

			command: u16,
			status: u16,

			revision_id: u8,
			prog_if: u8,
			subclass: u8,
			class_code: u8,

			cache_line_size: u8,
			latency_timer: u8,
			header_type: u8,
			bist: u8,

			base_address: [u32; 6],

			cardbus_cis_pointer: u32,

			subsystem_vendor_id: u16,
			subsystem_id: u16,

			expansion_rom_base_address: u32,

			capabilities_pointer: u8,
			_reserved_0: u8,
			_reserved_1: u16,

			_reserved_2: u32,

			interrupt_line: u8,
			interrupt_pin: u8,
			min_grant: u8,
			max_latency: u8,
		}

		//const _SIZE_CHECK: usize = 64 - core::mem::size_of::<PCIHeader>();

		// Bruteforce scan bus
		let a = 0x3000_0000_u32;
		for bus in 0..255 {
			for dev in 0..32 {
				for func in 0..8 {
					let a = a | (bus << 20) | (dev << 15) | (func << 12);
					let mut pci = core::ptr::NonNull::new(a as *mut _).unwrap();
					let pci: &mut PCIHeader = pci.as_mut();
					if pci.vendor_id != 0xffff {
						log!("bus: {}, dev: {}, func: {}", bus, dev, func);
						dbg!(&pci);
						for (i, ba) in pci.base_address.iter_mut().enumerate() {
							if *ba == 0 {
								continue;
							}
							let ba = ba as *mut _;
							if *ba & 0x1_u32 == 0u32 {
								log!("{} Memory space", i);
							} else {
								log!("{} I/O space", i);
							}
							if *ba & 0x6_u32 == 0u32 {
								log!("  32 bit");
							} else {
								log!("  64 bit");
							}
							let o = ptr::read_volatile(ba);
							log!("    start: {:x}", o & 0xffff_fff0_u32);
							ptr::write_volatile(ba, 0xffff_ffff_u32);
							let v = ptr::read_volatile(ba);
							log!("    size : {:x}", !(v & 0xffff_fff0_u32) + 1);
							ptr::write_volatile(ba, o);
						}
					}
				}
			}
		}

		#[repr(C)]
		#[derive(Debug)]
		struct VirtioHeader {
			device_features: u32,
			guest_features: u32,
			queue_address: u32,
			queue_size: u16,
			queue_select: u16,
			queue_notify: u16,
			device_status: u8,
			isr_status: u8,
		}

		#[repr(C)]
		#[derive(Debug)]
		struct VirtioBlockDevice {
			header: VirtioHeader,
			total_sector_count: [u32; 2], // Not properly aligned :(
			maximum_segment_size: u32,
			maximum_segment_count: u32,
			cylinder_count: u16,
			head_count: u8,
			sector_count: u8,
			block_length: u8,
		}

		#[repr(C)]
		#[derive(Debug)]
		struct VirtioPCICapabilities {
			vendor: u8,
			next: u8,
			len: u8,
			typ: u8,
			bar: u8,
			padding: [u8; 3],
			offset: u32,
			length: u32,
			more_stuff: u32,
		}

		// Bus master host bridge
		log!("");
		log!("");
		let pci: &mut PCIHeader = NonNull::new(a as *mut _).unwrap().as_mut();
		ptr::write_volatile(&mut pci.command as *mut _, 0b00_0000_0110_u16);
		log!("");
		log!("");

		// Interact with SCSI virtio disk
		use core::ptr::NonNull;
		let pci: &mut PCIHeader = NonNull::new((a + (1 << 15)) as *mut _).unwrap().as_mut();
		//let mem_addr = 0x543_0000_u32;
		log!("");
		log!("");
		log!("");
		log!("BUS MASTER | MEM SPACE");
		ptr::write_volatile(&mut pci.command as *mut _, 0b00_0000_0110_u16);
		//ptr::write_volatile(&mut pci.base_address[4] as *mut _, 0x540_0000_u32);
		ptr::write_volatile(&mut pci.base_address[4] as *mut _, 0x4000_0000_u32);
		ptr::write_volatile(&mut pci.base_address[5] as *mut _, 0x0000_0000_u32);
		dbg!(ptr::read_volatile(pci as *mut _));
		let mem_addr = ptr::read_volatile(&mut pci.base_address[4] as *mut _);
		let mem_addr =             (ptr::read_volatile(&mut pci.base_address[4] as *mut _) & 0xffff_fff0_u32) as u64;
		let mem_addr = mem_addr | ((ptr::read_volatile(&mut pci.base_address[5] as *mut _) & 0xffff_ffff_u32) as u64) << 32;
		/*
		dbg!(ptr::read_volatile(pci as *const _));
		log!("");
		log!("BAR0+0x12 = 0x0");
		ptr::write_volatile((io_addr + 0x12) as *mut _, 0x0_u8);
		log!("");
		log!("BAR0+0x12 = 0x1");
		ptr::write_volatile((io_addr + 0x12) as *mut _, 0x1_u8);
		log!("");
		log!("BAR0+0x12 = 0x3");
		ptr::write_volatile((io_addr + 0x12) as *mut _, 0x3_u8);
		//dbg!(ptr::read_volatile(&mut pci.base_address[4] as *mut _) as *mut ());
		//dbg!(ptr::read_volatile(&mut pci.base_address[0] as *mut _) as *mut ());
		*/
		log!("");
		log!("");

		//dbg!(pci);

		let mut offset: u32 = u32::from(pci.capabilities_pointer);
		while offset != 0 {
			let vpc: u32 = a + (1 << 15) + offset;
			let vpc = NonNull::new(vpc as *mut VirtioPCICapabilities).unwrap().as_ref();
			dbg!(vpc);
			offset = u32::from(vpc.next);
		}

		log!("");
		log!("");

		#[repr(C)]
		#[derive(Debug)]
		struct VirtioPCICommonConfiguration {
			device_feature_select: u32,
			device_feature: u32,
			driver_feature_select: u32,
			driver_feature: u32,

			msix_configuration: u16,
			queue_count: u16,

			device_status: u8,
			config_generation: u8,

			queue_select: u16,
			queue_size: u16,
			queue_msix_vector: u16,
			queue_enable: u16,
			queue_notify_off: u16,
			queue_descriptors: u64,
			queue_driver: u64,
			queue_device: u64,
		}

		let addr = NonNull::new(mem_addr as *mut ()).unwrap();
		let vpcc = NonNull::new(addr.as_ptr() as *mut VirtioPCICommonConfiguration).unwrap().as_mut();
		dbg!(&vpcc);
		ptr::write_volatile(&mut vpcc.device_status as *mut _, 0u8); // reset
		ptr::write_volatile(&mut vpcc.device_status as *mut _, 1u8); // ack
		ptr::write_volatile(&mut vpcc.device_status as *mut _, 3u8); // ack + driver

		ptr::write_volatile(&mut vpcc.device_feature_select as *mut _, 0u32);
		let mut features = 0u32;
		dbg!(vpcc.device_feature as *mut ());
		if vpcc.device_feature & (1 << 1) > 0{
			features |= 1 << 1;
			log!("SIZE_MAX");
		}
		if vpcc.device_feature & (1 << 2) > 0 {
			features |= 1 << 2;
			log!("SEG_MAX");
		}
		if vpcc.device_feature & (1 << 4) > 0 {
			features |= 1 << 4;
			log!("GEOMETRY");
		}
		if vpcc.device_feature & (1 << 5) > 0 {
			log!("RO");
		}
		if vpcc.device_feature & (1 << 6) > 0 {
			features |= 1 << 6;
			log!("BLK_SIZE");
		}
		if vpcc.device_feature & (1 << 9) > 0 {
			features |= 1 << 9;
			log!("FLUSH");
		}
		if vpcc.device_feature & (1 << 10) > 0 {
			features |= 1 << 10;
			log!("TOPOLOGY");
		}
		if vpcc.device_feature & (1 << 11) > 0 {
			features |= 1 << 11;
			log!("CONFIG_WCE");
		}
		if vpcc.device_feature & (1 << 13) > 0 {
			features |= 1 << 13;
			log!("DISCARD");
		}
		if vpcc.device_feature & (1 << 14) > 0 {
			features |= 1 << 14;
			log!("WRITE_ZEROES");
		}
		if vpcc.device_feature & (1 << 27) > 0 {
			features |= 1 << 27;
			log!("VIRTIO_F_ANY_LAYOUT");
		}
		if vpcc.device_feature & (1 << 28) > 0 {
			//features |= 1 << 28;
			log!("VIRTIO_F_EVENT_IDX");
		}
		if vpcc.device_feature & (1 << 29) > 0 {
			//features |= 1 << 29;
			log!("VIRTIO_F_INDIRECT_DESC");
		}
		dbg!(features as *mut ());

		ptr::write_volatile(&mut vpcc.driver_feature_select as *mut _, 0u32);
		ptr::write_volatile(&mut vpcc.driver_feature as *mut _, features);
		dbg!(ptr::read_volatile(vpcc as *mut _));
		ptr::write_volatile(&mut vpcc.device_status as *mut _, 11u8); // ack + driver + features_ok
		dbg!(ptr::read_volatile(&mut vpcc.device_status as *mut _) as *mut ());

		#[repr(C)]
		#[derive(Debug)]
		struct VirtioBlkGeometry {
			cylinders: u16,
			heads: u8,
			sectors: u8,
		}

		#[repr(C)]
		#[derive(Debug)]
		struct VirtioBlkTopology {
			physical_block_exp: u8,
			alignment_offset: u8,
			min_io_size: u16,
			opt_io_size: u32,
		}

		#[repr(C)]
		#[derive(Debug)]
		struct VirtioBlkConfig {
			capacity: u64,
			size_max: u32,
			seg_max: u32,
			geometry: VirtioBlkGeometry,
			blk_size: u32,
			topology: VirtioBlkTopology,
			writeback: u8,
			_unused_0: [u8; 3],
			max_discard_sectors: u32,
			max_discard_seg: u32,
			discard_sector_alignment: u32,
			max_write_zeroes_sectors: u32,
			max_write_zeroes_seg: u32,
			write_zeroes_may_unmap: u8,
			_unused_1: [u8; 3],
		}

		#[repr(C)]
		#[derive(Debug)]
		struct BlkRequestHeader {
			typ: u32,
			reserved: u32,
			sector: u64,
		}
		struct BlkRequestData {
			data: [u8; 512],
		}
		struct BlkRequestStatus {
			status: u8,
		}

		let blk_cfg = NonNull::new((mem_addr + 0x2000) as *mut VirtioBlkConfig).unwrap().as_mut();
		dbg!(blk_cfg);

		let notify = (mem_addr + 0x3000) as *mut u16;

		// Set up queue.
	
		let mut vq_desc  = NonNull::new(0x8300_0000 as *mut VirtqDesc ).unwrap();
		let mut vq_avail = NonNull::new(0x8300_1000 as *mut VirtqAvail).unwrap();
		let mut vq_used  = NonNull::new(0x8300_2000 as *mut VirtqUsed ).unwrap();
		let mut vq_blk_h = NonNull::new(0x8301_0000 as *mut BlkRequestHeader).unwrap();
		let mut vq_blk_d = NonNull::new(0x8301_1000 as *mut BlkRequestData).unwrap();
		let mut vq_blk_s = NonNull::new(0x8301_2000 as *mut BlkRequestStatus).unwrap();

		ptr::write_volatile(&mut vpcc.queue_descriptors as *mut _, vq_desc.as_ptr() as u64);
		ptr::write_volatile(&mut vpcc.queue_driver as *mut _, vq_avail.as_ptr() as u64);
		ptr::write_volatile(&mut vpcc.queue_device as *mut _, vq_used.as_ptr() as u64);

		dbg!("rethug");

		vq_desc.as_ptr().add(0).write(VirtqDesc {
			address: vq_blk_h.as_ptr() as u64,
			length: core::mem::size_of::<BlkRequestHeader>() as u32,
			flags: 1,
			next: 1,
		});
		vq_desc.as_ptr().add(1).write(VirtqDesc {
			address: vq_blk_d.as_ptr() as u64,
			length: core::mem::size_of::<BlkRequestData>() as u32,
			flags: 1,
			next: 2,
		});
		vq_desc.as_ptr().add(2).write(VirtqDesc {
			address: vq_blk_s.as_ptr() as u64,
			length: core::mem::size_of::<BlkRequestStatus>() as u32,
			flags: 2,
			next: 0,
		});
		vq_avail.as_ptr().write(VirtqAvail {
			flags: 1,
			index: 0,
			ring: [0, 1, 2, 0, 0, 0, 0, 0],
		});
		vq_used.as_ptr().write(VirtqUsed {
			flags: 1,
			index: 0,
			ring: [VirtqUsedElem {
				index: 0,
				length: 0,
			}; 8],
		});
		vq_blk_h.as_ptr().write(BlkRequestHeader {
			typ: 1,
			reserved: 0,
			sector: 0,
		});
		vq_blk_d.as_ptr().write(BlkRequestData {
			data: [0; 512],
		});
		vq_blk_s.as_ptr().write(BlkRequestStatus {
			status: 111,
		});
		for (i, c) in b"Hello, world!".iter().enumerate() {
			vq_blk_d.as_mut().data[i] = *c;
		}

		asm!("fence");

		ptr::write_volatile(&mut vpcc.queue_select as *mut _, 0u16);
		ptr::write_volatile(&mut vpcc.queue_size as *mut _, 8u16);
		ptr::write_volatile(&mut vpcc.queue_enable as *mut _, 1u16);

		ptr::write_volatile(&mut vpcc.device_status as *mut _, 15u8); // ack + driver + features_ok + driver_ok
		dbg!(ptr::read_volatile(&mut vpcc.device_status as *mut _) as *mut ());

		vq_avail.as_mut().index += 1;

		const VIRTQ_DESC_F_AVAIL: u16 = 1 << 7;
		const VIRTQ_DESC_F_USED: u16 = 1 << 15;

		#[repr(C)]
		#[derive(Debug)]
		struct VirtqDesc {
			address: u64,
			length: u32,
			flags: u16,
			next: u16,
		}

		#[repr(C)]
		#[derive(Debug)]
		struct VirtqAvail {
			flags: u16,
			index: u16,
			ring: [u16; 8],
			// Only for VIRTIO_F_EVENT_IDX
			//used_event: u16,
		}

		#[repr(C)]
		#[derive(Copy, Clone, Debug)]
		struct VirtqUsedElem {
			index: u32,
			length: u32,
		}

		#[repr(C)]
		#[derive(Debug)]
		struct VirtqUsed {
			flags: u16,
			index: u16,
			ring: [VirtqUsedElem; 8],
			//avail_event: u16,
		}

		#[repr(C)]
		#[derive(Debug)]
		struct Virtq {
			num: u32,
			desc: *mut VirtqDesc,
			avail: *mut VirtqAvail,
			used: *mut VirtqUsed,
		}

		vq_avail.as_mut().index += 0;
		log!("MAMAAAA");
		ptr::write_volatile(notify, 0u16);

		// The below shuts down QEMU. Nice :)
		//*(0x100_000 as *mut u32) = 0x5555;

		loop {
			powerstate::halt();
		}
	}


	arch::VirtualMemorySystem::clear_identity_maps();

	// Run init
	// SAFETY: a valid init pointer and size should have been passed by boot.s.
	let init = unsafe { core::slice::from_raw_parts(initfs, initfs_size) };
	let init = elf::create_task(init.as_ref());
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
		log!("Running {} test{}", tests.len(), if tests.len() == 1 { "" } else { "s" });
		arch::init();
		for f in tests {
			// Reinitialize the memory manager each time in case of leaks or something else
			// overwriting it's state.
			// Incredibly unsafe, but w/e.
			unsafe {
				memory::mem_add_range(MEMORY_MANAGER_ADDRESS,
						NonZeroUsize::new(256).unwrap(),
					);
			}
			f();
		}
		log!("Done");
	}
}
