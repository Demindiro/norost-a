#![no_std]
#![no_main]
#![feature(asm)]
#![feature(allocator_api)]
#![feature(global_asm)]
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

include!(concat!(env!("OUT_DIR"), "/list.rs"));

use kernel::sys_log;
use xmas_elf::ElfFile;

#[export_name = "main"]
fn main() {
	sys_log!("Listing binary addresses:");

	// SAFETY: all zeroes TaskSpawnMapping is valid.
	let mut mappings =
		unsafe { core::mem::MaybeUninit::<[kernel::TaskSpawnMapping; 8]>::zeroed().assume_init() };
	let mut i = 0;
	let mut pc = 0;

	for bin in BINARIES.iter() {
		sys_log!("  {:p}", bin);
		let elf = ElfFile::new(bin).unwrap();
		sys_log!("  {:#?}", elf.header);
		for ph in elf.program_iter() {
			sys_log!("");
			sys_log!("  Offset  : 0x{:x}", ph.offset());
			sys_log!("  VirtAddr: 0x{:x}", ph.virtual_addr());
			sys_log!("  PhysAddr: 0x{:x}", ph.physical_addr());
			sys_log!("  FileSize: 0x{:x}", ph.file_size());
			sys_log!("  Mem Size: 0x{:x}", ph.mem_size());
			sys_log!("  Flags   : {}", ph.flags());
			sys_log!("  Align   : 0x{:x}", ph.align());

			let mut offset = ph.offset() & !0xfff;
			let mut virt_a = ph.virtual_addr() & !0xfff;

			for _ in 0..((ph.file_size() + 0xfff) & !0xfff) / 0x1000 {
				let self_address = bin.as_ptr().wrapping_add(offset as usize) as *mut _;
				let flags = ph.flags();
				let flags = u8::from(flags.is_read()) << 0
					| u8::from(flags.is_write()) << 1
					| u8::from(flags.is_execute()) << 2;
				sys_log!("    {:p} -> 0x{:x} ({:b})", self_address, virt_a, flags);
				mappings[i] = kernel::TaskSpawnMapping {
					typ: 0,
					flags,
					task_address: virt_a as *mut _,
					self_address,
				};
				i += 1;
				offset += 0x1000;
				virt_a += 0x1000;
			}
		}
		pc = elf.header.pt2.entry_point() as usize;
	}

	sys_log!("Spawning task");

	let kernel::Return { status, value: id } =
		unsafe { kernel::task_spawn(mappings.as_ptr(), i, pc as *const _, core::ptr::null()) };

	assert_eq!(status, 0, "Failed to spawn task");

	sys_log!("Spawned task with ID {}", id);

	let _ = unsafe { kernel::io_wait(0, 0) };

	// Dummy write some stuff to the spawned task
	unsafe {
		let addr = 0xff00_0000 as *mut kernel::Page;
		let kernel::Return { status, .. } = kernel::mem_alloc(addr, 2, 0b11);
		assert_eq!(status, 0);
		let raw = addr.add(1).cast::<u8>();
		let addr = addr.cast();
		let kernel::Return { status, .. } = kernel::io_set_queues(addr, 1, addr.add(1), 1);
		assert_eq!(status, 0);
		let s = "echo Hello, MiniSH! I am Init!\n";
		for (i, c) in s.bytes().enumerate() {
			*raw.add(i) = c;
		}
		addr.write(kernel::ipc::Packet {
			opcode: Some(kernel::ipc::Op::Write.into()),
			priority: 0,
			flags: 0,
			id: 0,
			address: id,
			data: kernel::ipc::Data { raw },
			length: s.len(),
		});
	}

	loop {
		unsafe { kernel::io_wait(0, 0) };
	}
}
