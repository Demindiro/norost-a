#![no_std]
#![no_main]
#![feature(asm)]
#![feature(core_intrinsics)]
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

mod rtbegin;

use core::convert::TryInto;

#[derive(Clone, Copy)]
#[repr(C)]
struct RGBA8 {
	r: u8,
	g: u8,
	b: u8,
	a: u8,
}

impl RGBA8 {
	const fn rgb(r: u8, g: u8, b: u8) -> Self {
		Self::rgba(r, g, b, 255)
	}

	const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
		Self { r, g, b, a }
	}
}

#[export_name = "main"]
fn main() {
	// FIXME move this to rtbegin
	unsafe { dux::init() };

	// Wait for virtio_gpu driver to come online
	let address = loop {
		let name = b"virtio_gpu";
		let ret = unsafe { kernel::sys_registry_get(name.as_ptr(), name.len()) };
		if ret.status == 0 {
			break ret.value;
		}
		unsafe { kernel::io_wait(0) };
	};

	// Request draw buffer
	unsafe {
		dux::ipc::add_free_range(dux::Page::new_unchecked(0x3333_0000 as *mut _), 2048).unwrap()
	};

	const OP_OPEN: u8 = 128;
	const OP_FLUSH: u8 = 129;

	{
		*dux::ipc::transmit() = kernel::ipc::Packet {
			flags: 0,
			id: 0,
			offset: 0,
			opcode: core::num::NonZeroU8::new(OP_OPEN),
			uuid: kernel::ipc::UUID::new(1),
			data: None,
			length: 0,
			name: None,
			name_len: 0,
			address,
		};
	}

	let buffer = {
		let rx = dux::ipc::receive();
		assert_eq!(rx.address, address);
		let ptr = rx.data.unwrap().as_ptr().cast::<RGBA8>();
		let len = rx.length / core::mem::size_of::<RGBA8>();
		// SAFETY: while the device will read from it, only we will write to it.
		unsafe { core::slice::from_raw_parts_mut(ptr, len) }
	};

	let (w, h) = (64, 64);
	for x in 0..w {
		for y in 0..w {
			// Check if it's below-left of the '\' diagonal
			let a = if x * 1 < y * 1 {
				// Check if it's above-left of the '/' diagonal
				// sin(pi / 8) is the factor
				let y = 64 - y;
				if y * 261 > x * 100 {
					255
				} else {
					0
				}
			} else {
				0
			};
			buffer[x + y * w] = RGBA8 {
				r: 255,
				g: 255,
				b: 255,
				a,
			}
		}
	}

	// Add self to registry
	let name = "console";
	let ret = unsafe { kernel::sys_registry_add(name.as_ptr(), name.len(), usize::MAX) };
	assert_eq!(ret.status, 0, "failed to add self to registry");

	let (mut x, mut y) = (0, 0);
	let (w, h) = (800, 600);

	/*
	loop {
		use core::slice;

		let rx = dux::ipc::receive();
		match rx.opcode.map(|n| n.get()).unwrap_or(0) {
			op if op == kernel::ipc::Op::Write as u8 => {
				let data = unsafe {
					slice::from_raw_parts(rx.data.unwrap().as_ptr().cast::<u8>(), rx.length)
				};
				let mut iter = data.iter();
				let fg = RGBA8::rgb(255, 255, 255);
				let bg = RGBA8::rgb(0, 0, 0);
				while let Some(c) = iter.next() {
					match c {
						b'\n' => {
							cursor_x = 0;
							cursor_y += 1;
						}
						b'\r' => cursor_x = 0,
						b'\x1b' => {
							assert_eq!(iter.next(), Some(&b'['));
							match iter.next().unwrap() {
								b'2' => match iter.next().unwrap() {
									b'K' => {
										for x in 0..cursor_w {
											let (x, y) =
												(x * Letter::WIDTH, cursor_y * Letter::HEIGHT);
											letter::get(0).copy(x, y, buffer, w, h, fg, bg);
										}
										cursor_x = 0;
									}
									_ => panic!(),
								},
								_ => panic!(),
							}
						}
						c => {
							let (x, y) = (cursor_x * Letter::WIDTH, cursor_y * Letter::HEIGHT);
							letter::get(*c).copy(x, y, buffer, w, h, fg, bg);
							cursor_x += 1;
							if cursor_x >= cursor_w {
								cursor_x = 0;
								cursor_y += 1;
							}
						}
					}
				}
				*dux::ipc::transmit() = kernel::ipc::Packet {
					flags: 0,
					id: rx.id,
					opcode: rx.opcode,
					offset: 0,
					uuid: kernel::ipc::UUID::INVALID,
					data: None,
					length: rx.length,
					name: None,
					name_len: 0,
					address: rx.address,
				};
			}
			_ => todo!(),
		}
		drop(rx);

		*dux::ipc::transmit() = kernel::ipc::Packet {
			flags: 0,
			id: 0,
			offset: 0,
			opcode: core::num::NonZeroU8::new(OP_FLUSH),
			uuid: kernel::ipc::UUID::INVALID,
			data: None,
			length: 0,
			name: None,
			name_len: 0,
			address,
		};
	}
	*/
	loop {
		unsafe { kernel::io_wait(1_000_000) };
	}
}
