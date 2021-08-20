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
	#[repr(C)]
	struct RGBA8 {
		r: u8,
		g: u8,
		b: u8,
		a: u8,
	}
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
			uuid: kernel::ipc::UUID::INVALID,
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

	let (w, h) = (800, 600);

	// Add self to registry
	let name = "console";
	let ret = unsafe { kernel::sys_registry_add(name.as_ptr(), name.len(), usize::MAX) };
	assert_eq!(ret.status, 0, "failed to add self to registry");

	let (mut a, mut b) = (1.0, 0.0);

	loop {
		//let rx = dux::ipc::receive();

		let d = 0.5;
		for x in 0u16..w {
			for y in 0u16..h {
				let r = f32::from(x) / f32::from(w) - d;
				let g = f32::from(y) / f32::from(h) - d;
				let (r, g) = (r * a - g * b, r * b + g * a);
				let i = usize::from(x) + usize::from(y) * usize::from(w);
				if (r * r + g * g) * 100.0 <= 25.0 {
					let (r, g) = (d + r, d + g);
					let r8 = (r * (255.0 / 2.0)) as u8;
					let g8 = (g * (255.0 / 2.0)) as u8;
					let (r, g) = (r8, g8);
					buffer[i] = RGBA8 {
						r: r * 2,
						g: g * 2,
						b: 255 - r - g,
						a: 255,
					};
				} else {
					buffer[i] = RGBA8 {
						r: 0,
						g: 0,
						b: 0,
						a: 255,
					};
				}
			}
		}

		let (x, y) = rotate(a, b);
		a = x;
		b = y;
		kernel::dbg!(x * x + y * y);

		use core::slice;

		/*
		match rx.opcode.map(|n| n.get()).unwrap_or(0) {
			_ => todo!(),
		}
		*/

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

		unsafe {
			kernel::io_wait(33_333);
		}
	}
}

const A: f32 = 255.0;
static B: f32 = 255.0;
static mut C: f32 = 255.0;

fn rotate(x: f32, y: f32) -> (f32, f32) {
	let r = 1.0 / 32.0;
	let (dx, dy) = (0.999, 0.0447101778122163142);
	let (x, y) = (x * dx - y * dy, x * dy + y * dx);
	let d = (1.0 - (x * x + y * y));
	// Perhaps not mathematically accurate, but it works
	(x - d * r, y - d * r)
}
