#![no_std]
#![no_main]
#![feature(asm)]
#![feature(allocator_api)]
#![feature(alloc_prelude)]
#![feature(default_alloc_error_handler)]
#![feature(global_asm)]
#![feature(naked_functions)]
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

static mut NEW_DATA: bool = false;

#[naked]
extern "C" fn notification_handler_entry() {
	unsafe {
		asm!(
			"
			# a0: type
			# a1: value
			# a7: address
			#
			# The original a[0-2] are stored on the stack by the kernel.
			.equ	GP_REGBYTES, 8
			.equ	NOTIFY_RETURN, 9
			addi	sp, sp, -(13 + 4) * GP_REGBYTES
			sd		t0, 0 * GP_REGBYTES (sp)
			sd		t1, 1 * GP_REGBYTES (sp)
			sd		t2, 2 * GP_REGBYTES (sp)
			sd		t3, 3 * GP_REGBYTES (sp)
			sd		t4, 4 * GP_REGBYTES (sp)
			sd		t5, 5 * GP_REGBYTES (sp)
			sd		t6, 6 * GP_REGBYTES (sp)
			sd		a3, 7 * GP_REGBYTES (sp)
			sd		a4, 8 * GP_REGBYTES (sp)
			sd		a5, 9 * GP_REGBYTES (sp)
			sd		a6, 10 * GP_REGBYTES (sp)
			sd		a2, 11 * GP_REGBYTES (sp)
			sd		ra, 12 * GP_REGBYTES (sp)
			mv		a2, a7
			call	notification_handler
			ld		t0, 0 * GP_REGBYTES (sp)
			ld		t1, 1 * GP_REGBYTES (sp)
			ld		t2, 2 * GP_REGBYTES (sp)
			ld		t3, 3 * GP_REGBYTES (sp)
			ld		t4, 4 * GP_REGBYTES (sp)
			ld		t5, 5 * GP_REGBYTES (sp)
			ld		t6, 6 * GP_REGBYTES (sp)
			ld		a3, 7 * GP_REGBYTES (sp)
			ld		a4, 8 * GP_REGBYTES (sp)
			ld		a5, 9 * GP_REGBYTES (sp)
			ld		a6, 10 * GP_REGBYTES (sp)
			ld		a2, 11 * GP_REGBYTES (sp)
			ld		ra, 12 * GP_REGBYTES (sp)
			addi	sp, sp, (13 + 4) * GP_REGBYTES
			li		a7, NOTIFY_RETURN
			ecall
		"
		);
	}
}

#[export_name = "notification_handler"]
extern "C" fn notification_handler(typ: usize, value: usize, address: usize) {
	let mut buf = [0; 16];
	let r = unsafe { CONSOLE.as_mut().unwrap().read(&mut buf) };
	unsafe {
		CONSOLE_BUFFER[CONSOLE_INDEX..CONSOLE_INDEX + r].copy_from_slice(&buf[..r]);
		CONSOLE_INDEX += r;
	}
}

static mut CONSOLE: Option<console::Console> = None;

static mut CONSOLE_BUFFER: [u8; 4096] = [0; 4096];
static mut CONSOLE_INDEX: usize = 0;

#[export_name = "main"]
fn main() {
	// GOD FUCKING DAMN IT RUST
	//
	// WHY ARE YOU STRIPPING THE __dux_init SYMBOL
	//
	// WHYYYYYYYYYYYYYYYY
	unsafe { dux::init() };

	sys_log!("Setting up notification handler");
	let ret = unsafe { kernel::io_set_notify_handler(notification_handler_entry) };
	assert_eq!(ret.status, 0);

	sys_log!("Mapping devices");
	device_tree::map_devices();
	pci::init_blk_device();

	let ret = unsafe { kernel::sys_reserve_interrupt(0xa) };
	assert_eq!(ret.status, 0, "failed to reserve interrupt");

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

	sys_log!("Press any key for magic");

	unsafe { CONSOLE = Some(console) };

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

	// Allocate a single page for transmitting data.
	let raw = dux::mem::reserve_range(None, 1)
		.unwrap()
		.as_ptr()
		.cast::<u8>();
	let ret = unsafe { kernel::mem_alloc(raw.cast(), 1, 0b011) };
	assert_eq!(ret.status, 0);

	// Add self to registry
	let name = "init_b0_test";
	let ret = unsafe { kernel::sys_registry_add(name.as_ptr(), name.len(), usize::MAX) };
	assert_eq!(ret.status, 0, "failed to add registry entry");

	// Check if we can find the added entry.
	let ret = unsafe { kernel::sys_registry_get(name.as_ptr(), name.len()) };
	assert_eq!(ret.status, 0, "failed to get registry entry");
	kernel::dbg!(ret.value);

	loop {
		// Wait for packets.
		let rxq = dux::ipc::receive();
		let op = rxq.opcode.unwrap();
		match kernel::ipc::Op::try_from(op) {
			Ok(kernel::ipc::Op::Read) => {
				// Figure out object to read.
				let data = unsafe {
					core::slice::from_raw_parts_mut(rxq.data.unwrap().as_ptr().cast(), rxq.length)
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
					let read = loop {
						let read = unsafe { CONSOLE_INDEX };
						if read > 0 {
							break read;
						}
						// TODO it should just be put in a queue..
						unsafe { kernel::io_wait(u64::MAX) };
					};
					unsafe {
						for i in 0..read {
							data[i] = CONSOLE_BUFFER[i];
						}
						CONSOLE_INDEX = 0;
					}
					data[..read]
						.iter_mut()
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
				let len = dux::Page::min_pages_for_range(rxq.length);
				let ret = unsafe { kernel::mem_dealloc(data.as_ptr() as *mut _, len) };
				assert_eq!(ret.status, 0);
				dux::ipc::add_free_range(
					dux::Page::new(core::ptr::NonNull::new(data.as_ptr() as *mut _).unwrap())
						.unwrap(),
					len,
				)
				.unwrap();
				if let Some(name) = rxq.name {
					let len = dux::Page::min_pages_for_range(rxq.name_len.into());
					let ret = unsafe { kernel::mem_dealloc(name.as_ptr() as *mut _, len) };
					assert_eq!(ret.status, 0);
					dux::ipc::add_free_range(
						dux::Page::new(core::ptr::NonNull::new(name.as_ptr() as *mut _).unwrap())
							.unwrap(),
						len,
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
					unsafe { CONSOLE.as_mut().unwrap().write(data) };
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
				let len = dux::Page::min_pages_for_range(rxq.length);
				let ret = unsafe { kernel::mem_dealloc(rxq.data.unwrap().as_ptr(), len) };
				assert_eq!(ret.status, 0);
				dux::ipc::add_free_range(
					dux::Page::new(core::ptr::NonNull::new(data.as_ptr() as *mut _).unwrap())
						.unwrap(),
					len,
				)
				.unwrap();
				if let Some(name) = rxq.name {
					let len = dux::Page::min_pages_for_range(rxq.name_len.into());
					let ret = unsafe { kernel::mem_dealloc(name.as_ptr() as *mut _, len) };
					assert_eq!(ret.status, 0);
					dux::ipc::add_free_range(
						dux::Page::new(core::ptr::NonNull::new(name.as_ptr() as *mut _).unwrap())
							.unwrap(),
						len,
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
				// FIXME Ultra shitty workaround to make sure we don't deallocate the pages
				// before they're transmitted.
				let _ = unsafe { kernel::io_wait(u64::MAX) };
			}
			Ok(op) => sys_log!("TODO {:?}", op),
			Err(kernel::ipc::UnknownOp) => sys_log!("Unknown op {}", op),
		}
	}
}
