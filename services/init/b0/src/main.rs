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

include!(concat!(env!("OUT_DIR"), "/list.rs"));

use core::fmt::Write;
use xmas_elf::ElfFile;

#[export_name = "main"]
fn main() {
    let _ = writeln!(kernel::SysLog, "Listing binary addresses:");

    // SAFETY: all zeroes TaskSpawnMapping is valid.
    let mut mappings =
        unsafe { core::mem::MaybeUninit::<[kernel::TaskSpawnMapping; 8]>::zeroed().assume_init() };
    let mut i = 0;
	let mut pc = 0;

    for bin in BINARIES.iter() {
        let _ = writeln!(kernel::SysLog, "  {:p}", bin);
        let elf = ElfFile::new(bin).unwrap();
        let _ = writeln!(kernel::SysLog, "  {:#?}", elf.header);
        for ph in elf.program_iter() {
            let _ = writeln!(kernel::SysLog, "");
            let _ = writeln!(kernel::SysLog, "  Offset  : 0x{:x}", ph.offset());
            let _ = writeln!(kernel::SysLog, "  VirtAddr: 0x{:x}", ph.virtual_addr());
            let _ = writeln!(kernel::SysLog, "  PhysAddr: 0x{:x}", ph.physical_addr());
            let _ = writeln!(kernel::SysLog, "  FileSize: 0x{:x}", ph.file_size());
            let _ = writeln!(kernel::SysLog, "  Mem Size: 0x{:x}", ph.mem_size());
            let _ = writeln!(kernel::SysLog, "  Flags   : {}", ph.flags());
            let _ = writeln!(kernel::SysLog, "  Align   : 0x{:x}", ph.align());

            let mut offset = ph.offset() & !0xfff;
            let mut virt_a = ph.virtual_addr() & !0xfff;

            for _ in 0..((ph.file_size() + 0xfff) & !0xfff) / 0x1000 {
                let self_address = bin.as_ptr().wrapping_add(offset as usize) as *mut _;
                let flags = ph.flags();
                let flags = u8::from(flags.is_read()) << 0
                    | u8::from(flags.is_write()) << 1
                    | u8::from(flags.is_execute()) << 2;
                let _ = writeln!(
                    kernel::SysLog,
                    "    {:p} -> 0x{:x} ({:b})",
                    self_address,
                    virt_a,
                    flags
                );
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

    let _ = writeln!(kernel::SysLog, "Spawning task");

    let kernel::Return { status, value } = unsafe {
        kernel::task_spawn(
            mappings.as_ptr(),
			i,
            pc as *const _,
            core::ptr::null(),
        )
    };

    assert_eq!(status, 0, "Failed to spawn task");

    let _ = writeln!(kernel::SysLog, "Spawned task with ID {}", value);

    loop {
        unsafe { kernel::io_wait(0, 0) };
    }
}
