//! # UART driver
//!
//! All this driver does is buffer UART input and send received data over it. It is meant to be
//! used by one task only.
//!
//! The driver does not add itself to the registry! This must be done by the "parent" task.

#![no_std]
#![no_main]
#![feature(asm)]
#![feature(global_asm)]
#![feature(panic_info_message)]

#[panic_handler]
fn panic_handler(_info: &core::panic::PanicInfo) -> ! {
	kernel::sys_log!("Panic!");
	#[cfg(debug_assertions)]
	if let Some(m) = _info.message() {
		kernel::sys_log!("  Message: {}", m);
	}
	loop {}
}

mod rtbegin;

use core::convert::{TryFrom, TryInto};
use core::ptr;
use core::str;

#[export_name = "main"]
extern "C" fn main(argc: usize, argv: *const *const u8) {
	let mut args = rtbegin::args(argc, argv);
	let mut reg = None;

	let ret = driver::parse_args(args, |arg, _| match arg {
		driver::Arg::Reg(r) => {
			if reg.replace(r).is_some() {
				kernel::sys_log!("--reg specified multiple times");
				exit_err();
			}
		}
		arg => {
			kernel::sys_log!("invalid argument {:?}", arg);
			exit_err();
		}
	});

	if let Err(e) = ret {
		kernel::sys_log!("error parsing arguments: {:?}", e);
		exit_err();
	}

	let reg = match reg {
		Some(reg) => reg,
		None => {
			kernel::sys_log!("--reg not specified");
			exit_err();
		}
	};

	let ps = u128::try_from(kernel::Page::SIZE).unwrap();
	let addr = match usize::try_from(reg.address / ps) {
		Ok(a) => a,
		Err(_) => {
			kernel::sys_log!("address out of range");
			exit_err();
		}
	};
	let size = match usize::try_from(reg.size / ps) {
		Ok(s) => s,
		Err(_) => {
			kernel::sys_log!("size out of range");
			exit_err();
		}
	};

	let ret = unsafe { kernel::sys_set_interrupt_controller(addr, size, 1023) };
	if ret.status != 0 {
		kernel::sys_log!("failed to set interrupt controller");
	}

	// We _still_ can't exit yet
	loop {
		unsafe { kernel::io_wait(u64::MAX) };
	}
}

fn exit_err() -> ! {
	// Ditto
	loop {
		unsafe { kernel::io_wait(u64::MAX) };
	}
}
