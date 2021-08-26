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
	exit_err_msg("Panic!");
}

mod rtbegin;

use core::convert::{TryFrom, TryInto};
use core::fmt::Write;
use core::ptr;
use core::str;

#[export_name = "main"]
extern "C" fn main(argc: usize, argv: *const *const u8) {
	let mut args = rtbegin::args(argc, argv);
	let mut reg = None;

	let ret = driver::parse_args(args, |arg, _| match arg {
		driver::Arg::Reg(r) => {
			if reg.replace(r).is_some() {
				exit_err_msg("--reg specified multiple times");
			}
		}
		arg => {
			let a = arg.cmd_arg().map(str::as_bytes).unwrap_or_else(|a| a);
			exit_err_msg_val("invalid argument ", a);
		}
	});

	if let Err(e) = ret {
		exit_err_msg_val("error parsing arguments ", e.as_str().as_bytes());
	}

	let reg = match reg {
		Some(reg) => reg,
		None => exit_err_msg("--reg not specified"),
	};

	let ps = u128::try_from(kernel::Page::SIZE).unwrap();
	let addr = match usize::try_from(reg.address / ps) {
		Ok(a) => a,
		Err(_) => exit_err_msg("address out of range"),
	};
	let size = match usize::try_from(reg.size / ps) {
		Ok(s) => s,
		Err(_) => exit_err_msg("size out of range"),
	};

	let ret = unsafe { kernel::sys_set_interrupt_controller(addr, size, 1023) };
	if ret.status != 0 {
		exit_err_msg("failed to set interrupt controller");
	}

	// We _still_ can't exit yet
	loop {
		unsafe { kernel::io_wait(u64::MAX) };
	}
}

fn exit_err_msg(msg: &str) -> ! {
	let _ = kernel::SysLog.write_str(msg);
	exit_err()
}

fn exit_err_msg_val(msg: &str, val: &[u8]) -> ! {
	let _ = kernel::SysLog.write_str(msg);
	let _ = unsafe { kernel::sys_log(val.as_ptr(), val.len()) };
	exit_err()
}

fn exit_err() -> ! {
	// Ditto
	loop {
		unsafe { kernel::io_wait(u64::MAX) };
	}
}
