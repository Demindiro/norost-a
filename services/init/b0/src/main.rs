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
use xmas_elf::ElfFile;

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
			let mut args = [&[][..]; 64];
			let mut argc = 0;

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

			// Push reg
			for &(a, s) in dev.reg {
				let (b, a) = fmt(buf, a);
				let (b, s) = fmt(b, s);
				args[argc] = b"--reg";
				args[argc + 1] = a;
				args[argc + 2] = s;
				argc += 3;
				buf = b;
			}

			// Push ranges
			for &(c, a, s) in dev.ranges {
				let (b, c) = fmt(buf, c);
				let (b, a) = fmt(b, a);
				let (b, s) = fmt(b, s);
				args[argc] = b"--range";
				args[argc + 1] = c;
				args[argc + 2] = a;
				args[argc + 3] = s;
				argc += 4;
				buf = b;
			}

			let address = dux::task::spawn_elf(data, &mut [].iter().copied(), &args[..argc])
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

	// Wait for uart to come online
	let uart_addr = loop {
		let name = b"uart";
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
					dux::task::Address::from(uart_addr),
					kernel::ipc::UUID::from(0x0),
				),
				(
					dux::task::Address::from(uart_addr),
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
