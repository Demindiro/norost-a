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
mod rtbegin;
mod uart;

include!(concat!(env!("OUT_DIR"), "/list.rs"));

#[global_allocator]
static FUCK_OFF: GlobalFuckOff = GlobalFuckOff;

struct GlobalFuckOff;

unsafe impl alloc::alloc::GlobalAlloc for GlobalFuckOff {
	unsafe fn alloc(&self, _layout: core::alloc::Layout) -> *mut u8 { todo!("Fuck off") }
	unsafe fn dealloc(&self, ptr: *mut u8, _layout: core::alloc::Layout) { todo!("Fuck off") }
}

use core::convert::TryFrom;
use kernel::sys_log;
use xmas_elf::ElfFile;

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

	sys_log!("Opening FAT FS");
	let mut dev = unsafe { pci::BLK.as_mut().unwrap().downcast_mut().unwrap() };
	let mut fs = match fs::open(virtio_block::Proxy::new(dev)) {
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
	let mut i = 0;
	while false {
		if let Some(c) = console.uart.read() {
			// TODO why tf does QEMU send a CR to us instead of LF?
			if c == b'\r' {
				use core::fmt::Write;
				writeln!(console);

				let buf = &buf[..i];
				match core::str::from_utf8(buf) {
					Ok(buf) => {
						let mut args = buf.split(|c: char| c == ' ').filter(|a| !a.is_empty());
						if let Some(cmd) = args.next() {
							match cmd {
								"echo" => {
									args.next().map(|a| console.write_str(a));
									for a in args {
										console.write_str(" ");
										console.write_str(a);
									}
									console.write_str("\n");
								}
								cmd => writeln!(console, "Unknown command '{}'", cmd).unwrap(),
							}
						}
					}
					Err(e) => writeln!(console, "Invalid command: {:?}", e).unwrap(),
				}

				i = 0;

				write!(console, "INIT >> ");
			} else {
				buf[i] = c;
				i += 1;
				console.uart.write(c);
			}
		}
	}

	sys_log!("Listing binary addresses:");

	// SAFETY: all zeroes TaskSpawnMapping is valid.
	let mut mappings =
		unsafe { core::mem::MaybeUninit::<[kernel::TaskSpawnMapping; 8]>::zeroed().assume_init() };
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
	let raw = dux::mem::reserve_range(None, 1).unwrap().as_ptr().cast::<u8>();
	let ret = unsafe { kernel::mem_alloc(raw.cast(), 1, 0b011) };
	assert_eq!(ret.status, 0);

	loop {
		// Read received data & write it to UART
		let _ = dux::ipc::try_receive(|rxq| {

			let op = rxq.opcode.unwrap();
			match kernel::ipc::Op::try_from(op) {
				Ok(kernel::ipc::Op::Read) => {

					// Figure out object to read.
					let data = unsafe { core::slice::from_raw_parts_mut(rxq.data.raw, rxq.length) };
					let path = rxq.name.map(|name| unsafe { core::slice::from_raw_parts(name.cast::<u8>().as_ptr(), rxq.name_len.into()) });

					// Read data
					let mut f = fs.root_dir().open_file(core::str::from_utf8(path.unwrap()).unwrap()).unwrap();
					use fatfs::Read;
					let len = f.read(data).unwrap();

					// Free ranges
					let _ = unsafe { kernel::mem_dealloc(data.as_ptr() as *mut _, 1) };
					dux::ipc::add_free_range(
						dux::Page::new(core::ptr::NonNull::new(data.as_ptr() as *mut _).unwrap()).unwrap(),
						1,
					);
					if let Some(name) = rxq.name {
						let _ = unsafe { kernel::mem_dealloc(name.as_ptr() as *mut _, 1) };
						dux::ipc::add_free_range(
							dux::Page::new(core::ptr::NonNull::new(name.as_ptr() as *mut _).unwrap()).unwrap(),
							1,
						);
					}

					// Send completion event
					dux::ipc::transmit(|pkt| *pkt = kernel::ipc::Packet {
						uuid: kernel::ipc::UUID::from(0),
						opcode: Some(kernel::ipc::Op::Write.into()),
						name: None,
						name_len: 0,
						flags: 0,
						id: 0,
						address: id,
						data: unsafe { kernel::ipc::Data { raw: core::ptr::null_mut() } },
						length: len,
						offset: 0,
					});
				}
				Ok(kernel::ipc::Op::Write) => {
					// Figure out object to write to.
					let data = unsafe { core::slice::from_raw_parts(rxq.data.raw, rxq.length) };
					let path = rxq.name.map(|name| unsafe { core::slice::from_raw_parts(name.cast::<u8>().as_ptr(), rxq.name_len.into()) });

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
						use fatfs::{Write, Seek, SeekFrom};
						f.seek(SeekFrom::Start(rxq.offset)).unwrap();
						f.write(data).unwrap()
					};

					// Free ranges
					let _ = unsafe { kernel::mem_dealloc(data.as_ptr() as *mut _, 1) };
					dux::ipc::add_free_range(
						dux::Page::new(core::ptr::NonNull::new(data.as_ptr() as *mut _).unwrap()).unwrap(),
						1,
					);
					if let Some(name) = rxq.name {
						let _ = unsafe { kernel::mem_dealloc(name.as_ptr() as *mut _, 1) };
						dux::ipc::add_free_range(
							dux::Page::new(core::ptr::NonNull::new(name.as_ptr() as *mut _).unwrap()).unwrap(),
							1,
						);

						// Confirm reception.
						dux::ipc::transmit(|pkt| *pkt = kernel::ipc::Packet {
							uuid: kernel::ipc::UUID::from(0),
							opcode: Some(kernel::ipc::Op::Write.into()),
							name: None,
							name_len: 0,
							flags: 0,
							id: 0,
							address: id,
							data: unsafe { kernel::ipc::Data { raw: core::ptr::null_mut() } },
							length: len,
							offset: 0,
						});
					}

					let _ = unsafe { kernel::mem_dealloc(data.as_ptr() as *mut _, 1) };
					dux::ipc::add_free_range(
						dux::Page::new(core::ptr::NonNull::new(data.as_ptr() as *mut _).unwrap()).unwrap(),
						1,
					);
				}
				Ok(kernel::ipc::Op::List) => {
					let mut list_builder = dux::ipc::list::Builder::new(fs.root_dir().iter().count(), 50).unwrap();
					for f in fs.root_dir().iter() {
						let f = f.unwrap();
						let uuid = kernel::ipc::UUID::from(0);
						let name = f.short_file_name_as_bytes();
						let size = f.len();
						list_builder.add(uuid, name, size).unwrap();
					}
					let list = dux::ipc::list::List::new(list_builder.data());

					let raw = list_builder.data() as *const _ as *mut _;

					let data = unsafe { core::slice::from_raw_parts(rxq.data.raw, rxq.length) };
					let _ = unsafe { kernel::mem_dealloc(data.as_ptr() as *mut _, 1) };
					dux::ipc::add_free_range(
						dux::Page::new(core::ptr::NonNull::new(data.as_ptr() as *mut _).unwrap()).unwrap(),
						1,
					);
					dux::ipc::transmit(|pkt| *pkt = kernel::ipc::Packet {
						uuid: kernel::ipc::UUID::from(0),
						opcode: Some(kernel::ipc::Op::List.into()),
						name: None,
						name_len: 0,
						flags: 0,
						id: rxq.id,
						address: id,
						data: unsafe { kernel::ipc::Data { raw } },
						length: list_builder.bytes_len(),
						offset: 0,
					});

					// FIXME goddamnit
					let _ = unsafe { kernel::io_wait(0, 0) };
					let _ = unsafe { kernel::io_wait(0, 0) };
				}
				Ok(op) => sys_log!("TODO {:?}", op),
				Err(kernel::ipc::UnknownOp) => sys_log!("Unknown op {}", op),
			}
		});

		// Read data from UART & send it to child process
		let mut buf = [0; 256];
		let read = console.read(&mut buf);
		let buf = &buf[..read];

		for (i, b) in buf.iter().copied().enumerate() {
			// UART pls
			unsafe { *raw.add(i) = if b == b'\r' { b'\n' } else { b } };
		}

		if buf.len() > 0 {
			dux::ipc::transmit(|pkt| *pkt = kernel::ipc::Packet {
				uuid: kernel::ipc::UUID::from(0),
				opcode: Some(kernel::ipc::Op::Write.into()),
				name: None,
				name_len: 0,
				flags: 0,
				id: 0,
				address: id,
				data: unsafe { kernel::ipc::Data { raw } },
				length: buf.len(),
				offset: 0,
			});
		}

		// Wait for more data & make sure our packet is sent.
		unsafe { kernel::io_wait(0, 0) };
	}
}
