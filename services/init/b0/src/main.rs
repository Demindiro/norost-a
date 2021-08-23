#![no_std]
#![no_main]
#![feature(asm)]
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

mod device_tree;
mod rtbegin;

include!(concat!(env!("OUT_DIR"), "/list.rs"));

use core::convert::TryFrom;
use kernel::sys_log;

#[export_name = "main"]
fn main() {
	unsafe { dux::init() };

	device_tree::iter_devices(|dev| {
		for bin in BINARIES.iter() {
			if !dev.compatible.contains(&bin.compatible.as_bytes()) {
				continue;
			}

			sys_log!(
				"Using driver {:?} for {:?}",
				bin.name,
				core::str::from_utf8(dev.name).unwrap()
			);

			// FIXME completely, utterly unsound
			let data = unsafe {
				core::slice::from_raw_parts(
					bin.data.as_ptr().cast(),
					(bin.data.len() + dux::Page::OFFSET_MASK) / dux::Page::SIZE,
				)
			};

			// Push arguments
			let mut buf = [0u8; 4096];
			let mut buf = &mut buf[..];
			let mut args = [&[][..]; 128];
			let mut argc = 0;

			fn alloc<'a>(
				buf: &'a mut [u8],
				size: usize,
			) -> Result<(&'a mut [u8], &'a mut [u8]), driver::OutOfMemory> {
				if size <= buf.len() {
					Ok(buf.split_at_mut(size))
				} else {
					Err(driver::OutOfMemory)
				}
			};
			let mut add_arg = |arg| {
				*args.get_mut(argc).ok_or(driver::OutOfMemory)? = str::as_bytes(arg);
				argc += 1;
				Ok(())
			};

			for &r in dev.reg.iter() {
				buf = r.to_args(buf, alloc, &mut add_arg).unwrap();
			}
			for &r in dev.ranges {
				buf = r.to_args(buf, alloc, &mut add_arg).unwrap();
			}
			for &im in dev.interrupt_map {
				buf = im.to_args(buf, alloc, &mut add_arg).unwrap();
			}
			if !dev.interrupt_map.is_empty() {
				dev.interrupt_map_mask.to_args(buf, alloc, &mut add_arg);
			}

			// Spawn
			let address = dux::task::spawn_elf(data, &mut [].iter().copied(), &args[..argc])
				.expect("failed to spawn task");

			sys_log!("Registering task {} as {:?}", address, bin.name);

			// Add to registry
			dux::task::registry::add(bin.name.as_bytes(), address)
				.expect("failed to add registry entry");

			return;
		}

		let _ = core::str::from_utf8(dev.name)
			.map(|name| sys_log!("No driver found for {:?}", name))
			.map_err(|_| sys_log!("No driver found for {:?}", dev.name));
		for c in dev.compatible {
			let _ = core::str::from_utf8(c)
				.map(|c| sys_log!("  {:?}", c))
				.map_err(|_| sys_log!("  {:?}", c));
		}
	});

	BINARIES
		.iter()
		.filter(|e| ["fs", "console"].contains(&e.compatible))
		.for_each(|e| {
			// FIXME completely, utterly unsound
			let data = unsafe {
				core::slice::from_raw_parts(
					e.data.as_ptr().cast(),
					(e.data.len() + dux::Page::OFFSET_MASK) / dux::Page::SIZE,
				)
			};
			// TODO which terminology to use? Ports seems... wrong?
			let ports = [];
			let ports = &mut ports.iter().copied();
			dux::task::spawn_elf(data, ports, &[]).expect("failed to spawn task");
		});

	// Wait for fatfs to come online
	let fatfs_addr = loop {
		let name = b"fatfs";
		let ret = unsafe { kernel::sys_registry_get(name.as_ptr(), name.len()) };
		if ret.status == 0 {
			break ret.value;
		}
		unsafe { kernel::io_wait(0) };
	};

	// Wait for uart / console to come online
	let uart_addr = loop {
		let name = b"uart";
		let name = b"QEMU Virtio Keyboard";
		let ret = unsafe { kernel::sys_registry_get(name.as_ptr(), name.len()) };
		if ret.status == 0 {
			break ret.value;
		}
		unsafe { kernel::io_wait(0) };
	};

	// Wait for uart / console to come online
	let console_addr = loop {
		let name = b"console";
		let ret = unsafe { kernel::sys_registry_get(name.as_ptr(), name.len()) };
		if ret.status == 0 {
			break ret.value;
		}
		unsafe { kernel::io_wait(0) };
	};

	BINARIES
		.iter()
		.filter(|e| e.compatible == "init")
		.for_each(|e| {
			// FIXME completely, utterly unsound
			let data = unsafe {
				core::slice::from_raw_parts(
					e.data.as_ptr().cast(),
					(e.data.len() + dux::Page::OFFSET_MASK) / dux::Page::SIZE,
				)
			};
			// TODO which terminology to use? Ports seems... wrong?
			let ports = [
				(
					dux::task::Address::from(uart_addr),
					kernel::ipc::UUID::from(0x0),
				),
				(
					dux::task::Address::from(console_addr),
					kernel::ipc::UUID::from(0x0),
				),
				(
					dux::task::Address::from(console_addr),
					kernel::ipc::UUID::from(0x0),
				),
				(
					dux::task::Address::from(fatfs_addr),
					kernel::ipc::UUID::from(0x0),
				),
			];
			let ports = &mut ports.iter().copied();
			dux::task::spawn_elf(data, ports, &[]).expect("failed to spawn task");
		});

	loop {
		// Do nothing as we can't exit
		unsafe { kernel::io_wait(u64::MAX) };
	}
}
