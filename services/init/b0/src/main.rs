#![no_std]
#![no_main]
#![feature(asm)]
#![feature(allocator_api)]
#![feature(alloc_prelude)]
#![feature(default_alloc_error_handler)]
#![feature(global_asm)]
#![feature(option_result_unwrap_unchecked)]
#![feature(panic_info_message)]
#![feature(ptr_metadata)]

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

extern crate alloc;

mod console;
mod device_tree;
mod fs;
mod pci;
mod plic;
mod rtbegin;
mod uart;

include!(concat!(env!("OUT_DIR"), "/list.rs"));

#[global_allocator]
static FUCK_OFF: GlobalFuckOff = GlobalFuckOff;

struct GlobalFuckOff;

unsafe impl alloc::alloc::GlobalAlloc for GlobalFuckOff {
	unsafe fn alloc(&self, _layout: core::alloc::Layout) -> *mut u8 {
		todo!("Fuck off")
	}
	unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {
		todo!("Fuck off")
	}
}

use core::convert::TryFrom;
use kernel::sys_log;
use xmas_elf::ElfFile;

extern "C" fn notification_handler(typ: usize, value: usize) {
	sys_log!("Got a notification!");
	sys_log!("  type  :    0x{:x}", typ);
	sys_log!("  value :    0x{:x}", value);
	// Do not crash do not crash do not crash if we return everything will explode
	loop {}
}

#[export_name = "main"]
fn main() {
	// GOD FUCKING DAMN IT RUST
	//
	// WHY ARE YOU STRIPPING THE __dux_init SYMBOL
	//
	// WHYYYYYYYYYYYYYYYY
	unsafe { dux::init() };

	sys_log!("Mapping devices");
	device_tree::map_devices();
	pci::init_blk_device();

	sys_log!("Setting up PLIC & enabling Interrupt 0");
	let mut plic = unsafe { plic::PLIC::new(device_tree::PLIC_ADDRESS) };
	let source = core::num::NonZeroU16::new(0xa).unwrap(); // UART
	let context = 0x1; // Hart 0 S-mode

	// The PLIC's behaviour should match that of SiFive's PLIC
	// https://static.dev.sifive.com/U54-MC-RVCoreIP.pdf
	// Presumably, since we're running on hart 0 (the only hart), we need to
	// enable the interrupt in context 0x1 (S-mode).
	plic.enable(context, source, true).unwrap();
	plic.set_priority(source, 5).unwrap();
	plic.set_priority_threshold(context, 6).unwrap();

	sys_log!("Opening FAT FS");
	let dev = unsafe { pci::BLK.as_mut().unwrap().downcast_mut().unwrap() };
	let fs = match fs::open(virtio_block::Proxy::new(dev)) {
		Ok(fs) => {
			sys_log!("Successfully opened FAT FS");
			fs
		}
		err => {
			// SAFETY: it's certainly an Err. It is done this way because the compiler doesn't
			// recognize we have ownership otherwise.
			let err = unsafe { err.unwrap_err_unchecked() };
			sys_log!("Failed to open FAT FS: {:?}", err);
			drop(err);
			sys_log!("Creating FAT FS");
			let fs = fs::init(virtio_block::Proxy::new(dev));
			sys_log!("Created FAT FS");
			fs
		}
	};

	sys_log!("Creating console");
	let mut console = unsafe { console::Console::new(device_tree::UART_ADDRESS.cast()) };
	let mut buf = [0; 256];

	sys_log!("Setting up notification handler");
	let ret = unsafe { kernel::io_set_notify_handler(notification_handler) };
	assert_eq!(ret.status, 0);

	sys_log!("Press any key for magic");
	let mut prev = None;
	plic.set_priority_threshold(context, 4).unwrap();

	sys_log!("Waiting for notification of stuff");
	loop {}

	loop {
	//for _ in 0..100 {
		let curr = plic.claim(context).unwrap();
		//plic.set_priority_threshold(context, 6).unwrap();
		if Some(curr) != prev {
			//kernel::dbg!(curr);
			prev = Some(curr);
		}
		/*
		if let Some(curr) = curr {
			plic.complete(context, curr).unwrap();
		}
		*/
		let r = console.read(&mut buf);
		if r > 0 {
			plic.complete(context, source).unwrap();
			//console.write(b"You typed '");
			//console.write(&buf[..r]);
			//console.write(b"'\n");
		}
	}

	sys_log!("Listing binary addresses:");

	// SAFETY: all zeroes TaskSpawnMapping is valid.
	let mut mappings =
		unsafe { core::mem::MaybeUninit::<[kernel::TaskSpawnMapping; 16]>::zeroed().assume_init() };
	let mut i = 0;
	let mut pc = 0;

	for bin in BINARIES.iter() {
		sys_log!("  {:p}", bin);
		let elf = ElfFile::new(bin).unwrap();
		for ph in elf.program_iter() {
			sys_log!("");
			sys_log!("  Offset  : 0x{:x}", ph.offset());
			sys_log!("  VirtAddr: 0x{:x}", ph.virtual_addr());
			sys_log!("  PhysAddr: 0x{:x}", ph.physical_addr());
			sys_log!("  FileSize: 0x{:x}", ph.file_size());
			sys_log!("  Mem Size: 0x{:x}", ph.mem_size());
			sys_log!("  Flags   : {}", ph.flags());
			sys_log!("  Align   : 0x{:x}", ph.align());

			let mut offset = ph.offset() & !0xfff;
			let mut virt_a = ph.virtual_addr() & !0xfff;

			let file_pages = ((ph.file_size() + 0xfff) & !0xfff) / 0x1000;
			let mem_pages = ((ph.mem_size() + 0xfff) & !0xfff) / 0x1000;
			let flags = ph.flags();
			let flags = u8::from(flags.is_read()) << 0
				| u8::from(flags.is_write()) << 1
				| u8::from(flags.is_execute()) << 2;

			if ph.flags().is_write() {
				// We must copy the pages as they may be written to.
				// FIXME add a sort of "mmap" to dux so we can avoid this lazy brokenness.
				let addr = 0xded_0000 as *mut _;
				let ret = unsafe { kernel::mem_alloc(addr, mem_pages as usize, flags) };
				let addr = addr.cast::<u8>();
				assert_eq!(ret.status, 0);
				let data = match ph {
					xmas_elf::program::ProgramHeader::Ph64(ph) => ph.raw_data(&elf),
					_ => unreachable!(),
				};
				for k in 0..ph.file_size() {
					unsafe { *addr.add(k as usize) = data[k as usize] };
				}
				for k in 0..mem_pages {
					let self_address = addr.wrapping_add((k * 0x1000) as usize) as *mut _;
					sys_log!("    {:p} -> 0x{:x} ({:b})", self_address, virt_a, flags);
					mappings[i] = kernel::TaskSpawnMapping {
						typ: 0,
						flags,
						task_address: virt_a as *mut _,
						self_address,
					};
					i += 1;
					offset += 0x1000;
					virt_a += 0x1000;
				}
			} else {
				// It is safe to share the pages
				for _ in 0..file_pages {
					let self_address = bin.as_ptr().wrapping_add(offset as usize) as *mut _;
					sys_log!("    {:p} -> 0x{:x} ({:b})", self_address, virt_a, flags);
					mappings[i] = kernel::TaskSpawnMapping {
						typ: 0,
						flags,
						task_address: virt_a as *mut _,
						self_address,
					};
					i += 1;
					offset += 0x1000;
					virt_a += 0x1000;
				}
			}
		}
		pc = elf.header.pt2.entry_point() as usize;
	}

	sys_log!("Spawning task");

	let kernel::Return { status, value: id } =
		unsafe { kernel::task_spawn(mappings.as_ptr(), i, pc as *const _, core::ptr::null()) };

	assert_eq!(status, 0, "Failed to spawn task");

	sys_log!("Spawned task with ID {}", id);

	// Create pseudo list.
	let mut list_builder = dux::ipc::list::Builder::new(fs.root_dir().iter().count(), 50).unwrap();
	for f in fs.root_dir().iter() {
		let f = f.unwrap();
		let uuid = kernel::ipc::UUID::from(0);
		let name = f.short_file_name_as_bytes();
		let size = f.len();
		list_builder.add(uuid, name, size).unwrap();
	}
	let list = dux::ipc::list::List::new(list_builder.data());
	sys_log!("Listing {} entries", list.iter().count());
	for (i, e) in list.iter().enumerate() {
		sys_log!("{}: {:?}", i, e);
	}
	drop(list);
	drop(list_builder);

	// Allocate a single page for transmitting data.
	let raw = dux::mem::reserve_range(None, 1)
		.unwrap()
		.as_ptr()
		.cast::<u8>();
	let ret = unsafe { kernel::mem_alloc(raw.cast(), 1, 0b011) };
	assert_eq!(ret.status, 0);

	loop {
		// Read received data & write it to UART
		if let Some(rxq) = dux::ipc::try_receive() {
			let op = rxq.opcode.unwrap();
			match kernel::ipc::Op::try_from(op) {
				Ok(kernel::ipc::Op::Read) => {
					// Figure out object to read.
					let data = unsafe {
						core::slice::from_raw_parts_mut(
							rxq.data.unwrap().as_ptr().cast(),
							rxq.length,
						)
					};
					let path = rxq.name.map(|name| unsafe {
						core::slice::from_raw_parts(name.cast::<u8>().as_ptr(), rxq.name_len.into())
					});

					let length = if let Some(path) = path {
						// Read data from file
						let mut f = fs
							.root_dir()
							.open_file(core::str::from_utf8(path).unwrap())
							.unwrap();
						use fatfs::Read;
						f.read(data).unwrap()
					} else {
						// Read data from UART
						let read = console.read(data);
						data.iter_mut()
							.filter(|b| **b == b'\r')
							.for_each(|b| *b = b'\n');
						read
					};

					// Send completion event
					*dux::ipc::transmit() = kernel::ipc::Packet {
						uuid: kernel::ipc::UUID::from(0x09090909090555577777),
						opcode: Some(kernel::ipc::Op::Read.into()),
						name: None,
						name_len: 0,
						flags: 0,
						id: 0,
						address: id,
						data: None,
						length,
						offset: 0,
					};

					// Free ranges
					let ret = unsafe { kernel::mem_dealloc(data.as_ptr() as *mut _, 1) };
					assert_eq!(ret.status, 0);
					dux::ipc::add_free_range(
						dux::Page::new(core::ptr::NonNull::new(data.as_ptr() as *mut _).unwrap())
							.unwrap(),
						dux::Page::min_pages_for_range(rxq.length),
					)
					.unwrap();
					if let Some(name) = rxq.name {
						let ret = unsafe { kernel::mem_dealloc(name.as_ptr() as *mut _, 1) };
						assert_eq!(ret.status, 0);
						dux::ipc::add_free_range(
							dux::Page::new(
								core::ptr::NonNull::new(name.as_ptr() as *mut _).unwrap(),
							)
							.unwrap(),
							dux::Page::min_pages_for_range(rxq.name_len.into()),
						)
						.unwrap();
					}
				}
				Ok(kernel::ipc::Op::Write) => {
					// Figure out object to write to.
					let data = unsafe {
						core::slice::from_raw_parts(rxq.data.unwrap().as_ptr().cast(), rxq.length)
					};
					let path = rxq.name.map(|name| unsafe {
						core::slice::from_raw_parts(name.cast::<u8>().as_ptr(), rxq.name_len.into())
					});

					// Write data
					let len = if path.is_none() {
						console.write(data);
						data.len()
					} else {
						let name = core::str::from_utf8(path.unwrap()).unwrap();
						let mut f = match fs.root_dir().open_file(name) {
							Ok(f) => f,
							Err(_) => fs.root_dir().create_file(name).unwrap(),
						};
						use fatfs::{Seek, SeekFrom, Write};
						f.seek(SeekFrom::Start(rxq.offset)).unwrap();
						f.write(data).unwrap()
					};

					// Free ranges
					let ret = unsafe { kernel::mem_dealloc(data.as_ptr() as *mut _, 1) };
					assert_eq!(ret.status, 0);
					dux::ipc::add_free_range(
						dux::Page::new(core::ptr::NonNull::new(data.as_ptr() as *mut _).unwrap())
							.unwrap(),
						dux::Page::min_pages_for_range(rxq.length),
					)
					.unwrap();
					if let Some(name) = rxq.name {
						let ret = unsafe { kernel::mem_dealloc(name.as_ptr() as *mut _, 1) };
						assert_eq!(ret.status, 0);
						dux::ipc::add_free_range(
							dux::Page::new(
								core::ptr::NonNull::new(name.as_ptr() as *mut _).unwrap(),
							)
							.unwrap(),
							dux::Page::min_pages_for_range(rxq.name_len.into()),
						)
						.unwrap();
					}

					// Confirm reception.
					*dux::ipc::transmit() = kernel::ipc::Packet {
						uuid: kernel::ipc::UUID::from(0x10101010101010),
						opcode: Some(kernel::ipc::Op::Write.into()),
						name: None,
						name_len: 0,
						flags: 0,
						id: 0,
						address: id,
						data: None,
						length: len,
						offset: 0,
					};
					unsafe { kernel::io_wait(0, 0) };
				}
				Ok(kernel::ipc::Op::List) => {
					let mut list_builder =
						dux::ipc::list::Builder::new(fs.root_dir().iter().count(), 50).unwrap();
					for f in fs.root_dir().iter() {
						let f = f.unwrap();
						let uuid = kernel::ipc::UUID::from(0);
						let name = f.short_file_name_as_bytes();
						let size = f.len();
						list_builder.add(uuid, name, size).unwrap();
					}

					let data = Some(core::ptr::NonNull::from(list_builder.data()).cast());

					*dux::ipc::transmit() = kernel::ipc::Packet {
						uuid: kernel::ipc::UUID::from(0x22222222222222),
						opcode: Some(kernel::ipc::Op::List.into()),
						name: None,
						name_len: 0,
						flags: 0,
						id: rxq.id,
						address: id,
						data,
						length: list_builder.bytes_len(),
						offset: 0,
					};

					// FIXME goddamnit
					let _ = unsafe { kernel::io_wait(0, 0) };
					let _ = unsafe { kernel::io_wait(0, 0) };
				}
				Ok(op) => sys_log!("TODO {:?}", op),
				Err(kernel::ipc::UnknownOp) => sys_log!("Unknown op {}", op),
			}
		}

		// Wait for more data & make sure our packet is sent.
		unsafe { kernel::io_wait(0, 0) };
	}
}
