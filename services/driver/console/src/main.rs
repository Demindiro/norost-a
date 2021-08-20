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

	let mut a = fcvt_s_lu(0);

	unsafe {
		kernel::dbg!(core::ptr::read_volatile(&A));
		kernel::dbg!(core::ptr::read_volatile(&B));
		kernel::dbg!(core::ptr::read_volatile(&C));
	}

	let (mut a, mut b) = (fcvt_s_lu(1), fcvt_s_lu(0));

	loop {
		//let rx = dux::ipc::receive();

		let fw = fcvt_s_lu(w);
		let fh = fcvt_s_lu(h);
		let d = fcvt_s_lu(1) / fcvt_s_lu(2);
		for x in 0..w {
			for y in 0..h {
				let r = fcvt_s_lu(x) / fw - d;
				let g = fcvt_s_lu(y) / fh - d;
				let (r, g) = (r * a - g * b, r * b + g * a);
				if (r * r + g * g) * fcvt_s_lu(100) <= fcvt_s_lu(25) {
					let (r, g) = (d + r, d + g);
					let r8 = fcvt_lu_s(r * fcvt_s_lu(127)) as u8;
					let g8 = fcvt_lu_s(g * fcvt_s_lu(127)) as u8;
					let (r, g) = (r8, g8);
					buffer[x + y * w] = RGBA8 {
						r: r * 2,
						g: g * 2,
						b: 255 - r - g,
						a: 255,
					};
				} else {
					buffer[x + y * w] = RGBA8 {
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
	//let (dx, dy) = (0.95, 0.31225);
	let r = fcvt_s_lu(1_000_000);
	let (dx, dy) = (fcvt_s_lu(999_000) / r, fcvt_s_lu(44_710) / r);
	let (x, y) = (x * dx - y * dy, x * dy + y * dx);
	let d = (1.0 - (x * x + y * y));
	(x - d / r, y - d / r)
}

// FIXME Rust's/LLVM codegen for float <-> int conversions is broken
//
// We should report this, but I have no idea how to create a minimal reproduction that others can use...
#[inline(always)]
fn fcvt_s_lu(n: usize) -> f32 {
	unsafe {
		let f: f32;
		asm!("fcvt.s.lu {0}, {1}", out(freg) f, in(reg) n);
		f
	}
}
#[inline(always)]
fn fcvt_lu_s(f: f32) -> usize {
	unsafe {
		let n: usize;
		asm!("fcvt.lu.s {0}, {1}", out(reg) n, in(freg) f);
		n
	}
}
