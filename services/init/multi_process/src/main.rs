#![no_std]
#![no_main]
#![feature(asm)]
#![feature(allocator_api)]
#![feature(global_asm)]
#![feature(panic_info_message)]
#![feature(ptr_metadata)]

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
	let _ = writeln!(kernel::SysLog, "Panic!");
	if let Some(m) = info.message() {
		let _ = writeln!(kernel::SysLog, "  Message: {}", m);
	}
	if let Some(l) = info.location() {
		let _ = writeln!(kernel::SysLog, "  Location: {}", l);
	}
	loop {}
}

global_asm!(
	"
	.globl	_start
	_start:
		li		a7, 3				# mem_alloc
		li		a0, 0xffff0000		# address
		li		a1, 0x10000 / 4096	# size (64K)
		li		a2, 0b011			# flags (RW)
		ecall

	0:
		bnez	a0, 0b				# Loop forever on error

		li		sp, 0xffffffff			# Set stack pointer

		addi	sp, sp, -8			# Set return address to 0 to aid debugger
		sd		zero, 0(sp)

		call	main

	0:
		j		0b				# Loop forever as we can't exit
	",
);

use core::fmt::Write;

global_asm!(
	"
	.align 12
	hello_world_task:
		li		a7, 15					# sys_call
		la		a0, hello_world_string	# address
		li		a1, 15					# size
		ecall
	0:
		j		0b				# Loop forever as we can't exit

	hello_world_string:
		.ascii \"Hello!\nI live!\n\"
	",
);

#[export_name = "main"]
fn main() {
	let _ = writeln!(kernel::SysLog, "Setting up task data");

	let addr;
	unsafe { asm!("la	{0}, hello_world_task", out(reg) addr) };

	let mappings = [kernel::TaskSpawnMapping {
		typ: 0,
		flags: 0b101,
		task_address: addr,
		self_address: addr,
	}];

	let _ = writeln!(kernel::SysLog, "Spawning task");

	let kernel::Return { status, value } = unsafe {
		kernel::task_spawn(
			mappings.as_ptr(),
			mappings.len(),
			addr.cast(),
			core::ptr::null(),
		)
	};

	assert_eq!(status, 0, "Failed to spawn task");

	let _ = writeln!(kernel::SysLog, "Spawned task with ID {}", value);

	unsafe { kernel::io_wait(0, 0) };
	loop {}
}
