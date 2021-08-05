#![no_std]
#![no_main]
#![feature(asm)]
#![feature(allocator_api)]
#![feature(alloc_prelude)]
#![feature(default_alloc_error_handler)]
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

extern crate alloc;

mod console;
mod device_tree;
mod fs;
mod pci;
mod rtbegin;
mod uart;

include!(concat!(env!("OUT_DIR"), "/list.rs"));

#[global_allocator]
static FUCK_OFF: GlobalFuckOff = GlobalFuckOff;

struct GlobalFuckOff;

unsafe impl alloc::alloc::GlobalAlloc for GlobalFuckOff {
	unsafe fn alloc(&self, _layout: core::alloc::Layout) -> *mut u8 { todo!("Fuck off") }
	unsafe fn dealloc(&self, ptr: *mut u8, _layout: core::alloc::Layout) { todo!("Fuck off") }
}

use core::convert::TryFrom;
use kernel::sys_log;
use xmas_elf::ElfFile;

#[export_name = "main"]
fn main() {

	// GOD FUCKING DAMN IT RUST
	//
	// WHY ARE YOU STRIPPING THE __dux_init SYMBOL
	//
	// WHYYYYYYYYYYYYYYYY
	unsafe { dux::init() };

	sys_log!("Mapping devices");
	device_tree::map_devices();
	pci::init_blk_device();

	sys_log!("Creating console");
	{
		use core::fmt::Write;
		let _ = write!(kernel::SysLog, "INIT >> ");
	}

	struct Dummy<'a>(&'a mut [u8], usize);

	impl fatfs::IoBase for Dummy<'_> {
		type Error = ();
	}
	impl fatfs::Read for Dummy<'_> {
		fn read(&mut self, data: &mut [u8]) -> Result<usize, Self::Error> {
			let max_r = self.0[self.1..].len().min(data.len());
			for (w, r) in data.iter_mut().zip(self.0[self.1..].iter().copied()) {
				*w = r;
			}
			self.1 += max_r;
			Ok(max_r)
		}
	}
	impl fatfs::Write for Dummy<'_> {
		fn write(&mut self, data: &[u8]) -> Result<usize, Self::Error> {
			let max_w = self.0[self.1..].len().min(data.len());
			for (w, r) in self.0[self.1..].iter_mut().zip(data.iter().copied()) {
				*w = r;
			}
			self.1 += max_w;
			Ok(max_w)
		}
		fn flush(&mut self) -> Result<(), Self::Error> {
			Ok(())
		}
	}
	impl fatfs::Seek for Dummy<'_> {
		fn seek(&mut self, pos: fatfs::SeekFrom) -> Result<u64, Self::Error> {
			use core::convert::TryInto;
			match pos {
				fatfs::SeekFrom::Start(p) => self.1 = p.try_into().unwrap(),
				fatfs::SeekFrom::Current(d) => {
					if d > 0 {
						self.1 += { let d: usize = d.try_into().unwrap(); d };
					} else {
						self.1 -= { let d: usize = (-d).try_into().unwrap(); d };
					}
				}
				fatfs::SeekFrom::End(d) => self.1 = self.0.len() - { let d: usize = (-d).try_into().unwrap(); d },
			}
			Ok(self.1.try_into().unwrap())
		}
	}
	impl fatfs::IoBase for &mut Dummy<'_> {
		type Error = ();
	}
	impl fatfs::Read for &mut Dummy<'_> {
		fn read(&mut self, data: &mut [u8]) -> Result<usize, Self::Error> {
			Dummy::read(self, data)
		}
	}
	impl fatfs::Write for &mut Dummy<'_> {
		fn write(&mut self, data: &[u8]) -> Result<usize, Self::Error> {
			Dummy::write(self, data)
		}
		fn flush(&mut self) -> Result<(), Self::Error> {
			Dummy::flush(self)
		}
	}
	impl fatfs::Seek for &mut Dummy<'_> {
		fn seek(&mut self, pos: fatfs::SeekFrom) -> Result<u64, Self::Error> {
			(*self).seek(pos)
		}
	}

	let mut buf = [0u8; 512 * 11];
	let mut fs = fs::init(Dummy(&mut buf[..], 0));

	let mut console = unsafe { console::Console::new(device_tree::UART_ADDRESS.cast()) };
	let mut buf = [0; 256];
	let mut i = 0;
	while false {
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

	dux::ipc::add_free_range(
		unsafe { dux::Page::new_unchecked(0x66_0000 as *mut _) },
		1,
	);

	// Create pseudo list.
	let mut list_builder = dux::ipc::list::Builder::new(3, 50).unwrap();
	for f in fs.root_dir().iter() {
		list_builder.add(kernel::ipc::UUID::from(0), f.unwrap().short_file_name_as_bytes()).unwrap();
	}
	let list = dux::ipc::list::List::new(list_builder.data());
	sys_log!("Listing {} entries", list.iter().count());
	for (i, e) in list.iter().enumerate() {
		sys_log!("{}: {:?}", i, e);
	}

	// Allocate a single page for transmitting data.
	let raw = dux::mem::reserve_range(None, 1).unwrap().as_ptr().cast::<u8>();
	let ret = unsafe { kernel::mem_alloc(raw.cast(), 1, 0b011) };
	assert_eq!(ret.status, 0);

	loop {
		// Read received data & write it to UART
		let _ = dux::ipc::try_receive(|rxq| {
			let op = rxq.opcode.unwrap();
			match kernel::ipc::Op::try_from(op) {
				Ok(kernel::ipc::Op::Write) => {
					let data = unsafe { core::slice::from_raw_parts(rxq.data.raw, rxq.length) };
					console.write(data);
					let _ = unsafe { kernel::mem_dealloc(data.as_ptr() as *mut _, 1) };
					dux::ipc::add_free_range(
						dux::Page::new(core::ptr::NonNull::new(data.as_ptr() as *mut _).unwrap()).unwrap(),
						1,
					);
				}
				Ok(kernel::ipc::Op::List) => {
					let raw = list_builder.data() as *const _ as *mut _;
					let data = unsafe { core::slice::from_raw_parts(rxq.data.raw, rxq.length) };
					let _ = unsafe { kernel::mem_dealloc(data.as_ptr() as *mut _, 1) };
					dux::ipc::add_free_range(
						dux::Page::new(core::ptr::NonNull::new(data.as_ptr() as *mut _).unwrap()).unwrap(),
						1,
					);
					dux::ipc::transmit(|pkt| *pkt = kernel::ipc::Packet {
						uuid: kernel::ipc::UUID::from(0),
						opcode: Some(kernel::ipc::Op::List.into()),
						flags: 0,
						id: rxq.id,
						address: id,
						data: unsafe { kernel::ipc::Data { raw } },
						length: buf.len(),
						offset: 0,
					});
				}
				Ok(op) => sys_log!("TODO {:?}", op),
				Err(kernel::ipc::UnknownOp) => sys_log!("Unknown op {}", op),
			}
		});

		// Read data from UART & send it to child process
		let mut buf = [0; 256];
		let read = console.read(&mut buf);
		let buf = &buf[..read];

		for (i, b) in buf.iter().copied().enumerate() {
			// UART pls
			unsafe { *raw.add(i) = if b == b'\r' { b'\n' } else { b } };
		}

		if buf.len() > 0 {
			dux::ipc::transmit(|pkt| *pkt = kernel::ipc::Packet {
				uuid: kernel::ipc::UUID::from(0),
				opcode: Some(kernel::ipc::Op::Write.into()),
				flags: 0,
				id: 0,
				address: id,
				data: unsafe { kernel::ipc::Data { raw } },
				length: buf.len(),
				offset: 0,
			});
		}

		// Wait for more data & make sure our packet is sent.
		unsafe { kernel::io_wait(0, 0) };
	}
}
