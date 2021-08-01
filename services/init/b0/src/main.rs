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

mod console;
mod rtbegin;
mod uart;

include!(concat!(env!("OUT_DIR"), "/list.rs"));

use kernel::sys_log;
use xmas_elf::ElfFile;

#[export_name = "main"]
fn main() {

	sys_log!("Scanning bus");
	le


	sys_log!("Creating console");
	{
		use core::fmt::Write;
		let _ = write!(kernel::SysLog, "INIT >> ");
	}

	let uart = core::ptr::NonNull::new(0x10000000 as *mut _).unwrap();

	let _ = unsafe { kernel::sys_direct_alloc(uart.as_ptr(), uart.as_ptr() as usize >> 12, 1, 0b011) };

	let mut console = unsafe { console::Console::new(uart.cast()) };
	let mut buf = [0; 256];
	let mut i = 0;
	loop {
		if let Some(c) = console.uart.read() {
			// TODO why tf does QEMU send a CR to us instead of LF?
			if c == b'\r' {
				use core::fmt::Write;
				writeln!(console);

				let buf = &buf[..i];
				match core::str::from_utf8(buf) {
					Ok(buf) => {
						let mut args = buf.split(|c: char| c == ' ').filter(|a| !a.is_empty());
						if let Some(cmd) = args.next() {
							match cmd {
								"echo" => {
									args.next().map(|a| console.write_str(a));
									for a in args {
										console.write_str(" ");
										console.write_str(a);
									}
									console.write_str("\n");
								}
								cmd => writeln!(console, "Unknown command '{}'", cmd).unwrap(),
							}
						}
					}
					Err(e) => writeln!(console, "Invalid command: {:?}", e).unwrap(),
				}

				i = 0;

				write!(console, "INIT >> ");
			} else {
				buf[i] = c;
				i += 1;
				console.uart.write(c);
			}
		}
	}

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
