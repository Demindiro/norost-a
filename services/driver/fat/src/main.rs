//! # FAT filesystem driver

#![no_std]
#![no_main]
#![feature(asm)]
#![feature(global_asm)]
#![feature(naked_functions)]
#![feature(panic_info_message)]

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
	kernel::sys_log!("Panic!");
	if let Some(m) = info.message() {
		kernel::sys_log!("  Message: {}", m);
	}
	if let Some(l) = info.location() {
		kernel::sys_log!("  Location: {}", l);
	}
	loop {}
}

use core::convert::TryFrom;

mod io;
mod rtbegin;

#[export_name = "main"]
fn main() {
	unsafe { dux::init() };

	// Wait for virtio_block driver to come online
	let addr = loop {
		let name = b"virtio_block";
		let ret = unsafe { kernel::sys_registry_get(name.as_ptr(), name.len()) };
		if ret.status == 0 {
			break ret.value;
		}
		unsafe { kernel::io_wait(0) };
	};

	unsafe { io::ADDRESS = addr };

	let mut buffer = kernel::Page::zeroed();
	let mut buffer2 = kernel::Page::zeroed();

	let fvo = fatfs::FormatVolumeOptions::new()
		.volume_label(*b"DUX ROOT\0\0\0")
		.volume_id(100117120)
		.max_root_dir_entries(16);
	let mut io = io::GlobalIO::new(&mut buffer);
	let fs = match fatfs::FileSystem::new(io, fatfs::FsOptions::new()) {
		Ok(fs) => fs,
		fs => {
			drop(fs);
			io = io::GlobalIO::new(&mut buffer2);
			fatfs::format_volume(&mut io, fvo).unwrap();
			let fs = fatfs::FileSystem::new(io, fatfs::FsOptions::new()).unwrap();
			use fatfs::Write;
			fs.root_dir()
				.create_file("ducks")
				.unwrap()
				.write(b"ducks\nducks ducks ducks ducks\nducks ducks ducks\nducks ducks\nducks")
				.unwrap();
			fs
		}
	};

	// Register self as fatfs filesystem
	let name = b"fatfs";
	let ret = unsafe { kernel::sys_registry_add(name.as_ptr(), name.len(), usize::MAX) };
	assert_eq!(ret.status, 0);

	loop {
		let rxq_lock = dux::ipc::receive();
		let rxq = (*rxq_lock).clone();
		drop(rxq_lock);
		let opcode = rxq.opcode.unwrap();

		use fatfs::{Read, Seek, SeekFrom, Write};

		match kernel::ipc::Op::try_from(opcode) {
			Ok(kernel::ipc::Op::Read) => {
				// Figure out object to read.
				let data = unsafe {
					core::slice::from_raw_parts_mut(rxq.data.unwrap().as_ptr().cast(), rxq.length)
				};
				let path = rxq.name.map(|name| unsafe {
					core::slice::from_raw_parts(name.cast::<u8>().as_ptr(), rxq.name_len.into())
				});

				let path = path.unwrap();
				let path = core::str::from_utf8(path).unwrap();
				let mut file = fs.root_dir().open_file(path).unwrap();
				file.seek(SeekFrom::Start(rxq.offset));
				let length = file.read(&mut data[..rxq.length]).unwrap();

				// Send completion event
				//*dux::ipc::transmit() = kernel::ipc::Packet {
				let mut tx = dux::ipc::transmit();
				*tx = kernel::ipc::Packet {
					uuid: kernel::ipc::UUID::INVALID,
					opcode: Some(opcode),
					name: None,
					name_len: 0,
					flags: 0,
					id: rxq.id,
					address: rxq.address,
					data: None,
					length,
					offset: rxq.offset,
				};
				// Drop now to prevent a deadlock
				drop(tx);
			}
			Ok(kernel::ipc::Op::Write) => {
				// Figure out object to write to.
				let data = unsafe {
					core::slice::from_raw_parts_mut(rxq.data.unwrap().as_ptr().cast(), rxq.length)
				};
				let path = rxq.name.map(|name| unsafe {
					core::slice::from_raw_parts(name.cast::<u8>().as_ptr(), rxq.name_len.into())
				});

				let path = path.unwrap();
				let path = core::str::from_utf8(path).unwrap();
				let mut file = fs.root_dir().create_file(path).unwrap();
				file.seek(SeekFrom::Start(rxq.offset));
				let length = file.write(&mut data[..rxq.length]).unwrap();

				// Confirm reception.
				let mut tx = dux::ipc::transmit();
				*tx = kernel::ipc::Packet {
					uuid: kernel::ipc::UUID::INVALID,
					opcode: Some(opcode),
					name: None,
					name_len: 0,
					flags: 0,
					id: rxq.id,
					address: rxq.address,
					data: None,
					length,
					offset: rxq.offset,
				};
				// Drop now to prevent a deadlock
				drop(tx);
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
					uuid: kernel::ipc::UUID::INVALID,
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
			// Just ignore other requests for now
			_ => (),
		}

		// Free ranges
		if let Some(data) = rxq.data {
			let len = dux::Page::min_pages_for_range(rxq.length);
			let ret = unsafe { kernel::mem_dealloc(data.as_ptr() as *mut _, len) };
			assert_eq!(ret.status, 0);
			dux::ipc::add_free_range(
				dux::Page::new(core::ptr::NonNull::new(data.as_ptr() as *mut _).unwrap()).unwrap(),
				len,
			)
			.unwrap();
		}
		if let Some(name) = rxq.name {
			let len = dux::Page::min_pages_for_range(rxq.name_len.into());
			let ret = unsafe { kernel::mem_dealloc(name.as_ptr() as *mut _, len) };
			assert_eq!(ret.status, 0);
			dux::ipc::add_free_range(
				dux::Page::new(core::ptr::NonNull::new(name.as_ptr() as *mut _).unwrap()).unwrap(),
				len,
			)
			.unwrap();
		}
	}
}
