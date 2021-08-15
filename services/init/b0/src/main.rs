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

mod device_tree;
mod fs;
mod pci;
mod rtbegin;

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

#[export_name = "main"]
fn main() {
	// GOD FUCKING DAMN IT RUST
	//
	// WHY ARE YOU STRIPPING THE __dux_init SYMBOL
	//
	// WHYYYYYYYYYYYYYYYY
	unsafe { dux::init() };

	device_tree::iter_devices(|name, compat, addr_size| {
		for bin in BINARIES.iter() {
			if !compat.contains(&bin.compatible.as_bytes()) {
				continue;
			}

			sys_log!(
				"Using driver {:?} for {:?}",
				bin.name,
				core::str::from_utf8(name).unwrap()
			);

			// FIXME completely, utterly unsound
			let data = unsafe {
				core::slice::from_raw_parts(
					bin.data.as_ptr().cast(),
					(bin.data.len() + dux::Page::OFFSET_MASK) / dux::Page::SIZE,
				)
			};
			// TODO which terminology to use? Ports seems... wrong?
			let mut buf = [0u8; 256];
			let mut buf = &mut buf[..];
			let mut args = [&[][..]; 8];
			let args = {
				let mut i = 0;
				fn fmt(buf: &mut [u8], mut num: u128) -> (&mut [u8], &mut [u8]) {
					let mut i = buf.len() - 1;
					while {
						let d = (num % 16) as u8;
						buf[i] = (d < 10).then(|| b'0').unwrap_or(b'a' - 10) + d;
						num /= 16;
						i -= 1;
						num != 0
					} {}
					buf.split_at_mut(i + 1)
				}
				for (i, (a, s)) in addr_size.iter().copied().enumerate() {
					let (b, a) = fmt(buf, a);
					let (b, s) = fmt(b, s);
					args[i * 2 + 0] = a;
					args[i * 2 + 1] = s;
					buf = b;
				}
				&args[..addr_size.len()]
			};
			let address = dux::task::spawn_elf(data, &mut [].iter().copied(), &args)
				.expect("failed to spawn task");

			// Allocate a single page for transmitting data.
			let raw = dux::mem::reserve_range(None, 1)
				.unwrap()
				.as_ptr()
				.cast::<u8>();
			let ret = unsafe { kernel::mem_alloc(raw.cast(), 1, 0b011) };
			assert_eq!(ret.status, 0);

			sys_log!("Registering task {} as {:?}", address, bin.name);

			// Add to registry
			dux::task::registry::add(bin.name.as_bytes(), address)
				.expect("failed to add registry entry");

			return;
		}

		let _ = core::str::from_utf8(name)
			.map(|name| sys_log!("No driver found for {:?}", name))
			.map_err(|_| sys_log!("No driver found for {:?}", name));
		for c in compat {
			let _ = core::str::from_utf8(c)
				.map(|c| sys_log!("  {:?}", c))
				.map_err(|_| sys_log!("  {:?}", c));
		}
	});

	pci::init_blk_device();

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

	BINARIES.iter().find(|e| e.compatible == "init").map(|e| {
		// FIXME completely, utterly unsound
		let data = unsafe {
			core::slice::from_raw_parts(
				e.data.as_ptr().cast(),
				(e.data.len() + dux::Page::OFFSET_MASK) / dux::Page::SIZE,
			)
		};
		let ports = [(dux::task::Address::from(1), kernel::ipc::UUID::from(0x0))];
		let ports = &mut ports.iter().copied();
		dux::task::spawn_elf(data, ports, &[]).expect("failed to spawn task");
	});

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

				let path = path.unwrap();
				// Read data from file
				let mut f = fs
					.root_dir()
					.open_file(core::str::from_utf8(path).unwrap())
					.unwrap();
				use fatfs::Read;
				let length = f.read(data).unwrap();

				// Send completion event
				*dux::ipc::transmit() = kernel::ipc::Packet {
					uuid: kernel::ipc::UUID::from(0x09090909090555577777),
					opcode: Some(kernel::ipc::Op::Read.into()),
					name: None,
					name_len: 0,
					flags: 0,
					id: 0,
					address: rxq.address,
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
				let name = core::str::from_utf8(path.unwrap()).unwrap();
				let mut f = match fs.root_dir().open_file(name) {
					Ok(f) => f,
					Err(_) => fs.root_dir().create_file(name).unwrap(),
				};
				use fatfs::{Seek, SeekFrom, Write};
				f.seek(SeekFrom::Start(rxq.offset)).unwrap();
				let len = f.write(data).unwrap();

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
					address: rxq.address,
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
					address: rxq.address,
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
