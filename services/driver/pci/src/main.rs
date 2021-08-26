#![no_std]
#![no_main]
#![feature(asm)]
#![feature(global_asm)]
#![feature(maybe_uninit_slice)]
#![feature(maybe_uninit_uninit_array)]
#![feature(naked_functions)]
#![feature(panic_info_message)]

use core::convert::{TryFrom, TryInto};
use core::mem::MaybeUninit;
use core::ptr::NonNull;
use core::str;

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

mod notification;
mod rtbegin;

include!(concat!(env!("OUT_DIR"), "/list.rs"));

#[derive(Clone, Copy)]
struct Task {
	address: dux::task::Address,
	child_address: u128,
}

#[derive(Clone, Copy)]
struct InterruptMap {
	bus: u16,
	system: u16,
	child_address: u128,
}

static mut TASKS: [Task; 16] = [Task {
	address: dux::task::Address::new(0),
	child_address: 0,
}; 16];
static mut TASKS_COUNT: usize = 0;

static mut INTERRUPT_MAP: [InterruptMap; 16] = [InterruptMap {
	bus: 0,
	system: 0,
	child_address: 0,
}; 16];
static mut INTERRUPT_MAP_COUNT: usize = 0;
static mut INTERRUPT_MAP_MASK: driver::InterruptMapMask = driver::InterruptMapMask::new(0, 0);

#[export_name = "main"]
fn main() {
	unsafe { dux::init() };

	let mut reg = None;
	let mut mmio = MaybeUninit::<pci::PhysicalMemory>::uninit_array::<8>();
	let mut mmio_count = 0;
	let mut unique_irqs = [0; 8];
	let mut unique_irqs_count = 0;

	driver::parse_args(rtbegin::args(), |arg, _| match arg {
		driver::Arg::Reg(r) => {
			reg.replace(r)
				.ok_or(())
				.expect_err("expecteed only one --reg specifier");
		}
		driver::Arg::Range(range) => {
			mmio[mmio_count].write(pci::PhysicalMemory {
				physical: usize::try_from(range.address).expect("physical address too large"),
				virt: NonNull::new(usize::MAX as *mut _).unwrap(),
				size: usize::try_from(range.size).expect("size too large"),
			});
			mmio_count += 1;
		}
		driver::Arg::InterruptMap(m) => unsafe {
			let system = m.parent_interrupt.try_into().unwrap();
			INTERRUPT_MAP[INTERRUPT_MAP_COUNT] = InterruptMap {
				bus: m.child_interrupt.try_into().unwrap(),
				system,
				child_address: m.child_address,
			};
			if !unique_irqs[..unique_irqs_count].contains(&system) {
				unique_irqs[unique_irqs_count] = system;
				unique_irqs_count += 1
			}
			INTERRUPT_MAP_COUNT += 1;
		},
		driver::Arg::InterruptMapMask(m) => unsafe { INTERRUPT_MAP_MASK = m },
		driver::Arg::Other(o) => panic!("unhandled {:?}", core::str::from_utf8(o)),
		_ => todo!(),
	});

	let reg = reg.expect("expecteed a --reg specifier");
	let addr = usize::try_from(reg.address).expect("address too large");
	let size = usize::try_from(reg.size).expect("size too large");

	// SAFETY: we properly initialized all the elements up to mmio_count
	let mmio = unsafe { MaybeUninit::slice_assume_init_mut(&mut mmio[..mmio_count]) };

	// Pick an address ourselves so the kernel can use hugepages.
	let mut virt = 0x1000_0000 as *mut _;

	let ret = unsafe {
		kernel::sys_direct_alloc(virt, addr / dux::Page::SIZE, size / dux::Page::SIZE, 0b11)
	};
	assert_eq!(ret.status, 0);
	let pci_virt = NonNull::new(virt).unwrap();
	virt = virt.wrapping_add(size / kernel::Page::SIZE);

	// Sort by size & align virt to help ensure optimal mapping by the kernel.
	mmio.sort_unstable_by(|a, b| b.size.cmp(&a.size));

	for m in mmio.iter_mut() {
		// Align virt for optimal mappping
		virt = match m.size {
			s if s >= 1 << 39 => virt.wrapping_add(virt.align_offset(1 << 39)),
			s if s >= 1 << 30 => virt.wrapping_add(virt.align_offset(1 << 30)),
			s if s >= 1 << 21 => virt.wrapping_add(virt.align_offset(1 << 21)),
			_ => virt,
		};
		// FIXME not implemented.
		//let virt = dux::mem::reserve_range(Some(virt), size / dux::Page::SIZE).unwrap();
		m.virt = NonNull::new(virt).unwrap();
		virt = virt.wrapping_add(m.size / kernel::Page::SIZE);
	}

	assert!(
		!mmio.is_empty(),
		"can't configure devices without MMIO area"
	);

	let pci = unsafe { pci::PCI::new(pci_virt, addr, size, mmio) };

	// FIXME christ
	let mut mmio = mmio[1].physical;

	for bus in pci.iter() {
		for dev in bus.iter() {
			let (v, d) = (dev.vendor_id(), dev.device_id());

			if let Some(bin) = BINARIES.iter().find(|b| b.vendor == v && b.device == d) {
				// FIXME completely, utterly unsound
				let data = unsafe {
					core::slice::from_raw_parts(
						bin.data.as_ptr().cast(),
						(bin.data.len() + dux::Page::OFFSET_MASK) / dux::Page::SIZE,
					)
				};
				kernel::sys_log!("Driver found for {:x}|{:x}", v, d);

				// Push arguments
				let mut buf = [0u8; 4096];
				let mut buf = &mut buf[..];
				let mut args = [&[][..]; 64];
				let mut argc = 0;

				fn alloc<'a>(
					buf: &'a mut [u8],
					size: usize,
				) -> Result<(&'a mut [u8], &'a mut [u8]), driver::OutOfMemory> {
					if size <= buf.len() {
						Ok(buf.split_at_mut(size))
					} else {
						Err(driver::OutOfMemory)
					}
				};
				let mut add_arg = |arg| {
					*args.get_mut(argc).ok_or(driver::OutOfMemory)? = str::as_bytes(arg);
					argc += 1;
					Ok(())
				};

				// Pass PCI MMIO area
				let child_address = u128::from(dev.child_address());
				let address = u128::try_from(dev.header_physical_address()).unwrap();
				let size = u128::try_from(dev.header().size()).unwrap();
				buf = driver::Pci {
					child_address,
					address,
					size,
				}
				.to_args(buf, &mut alloc, &mut add_arg)
				.unwrap();

				// Parse BARs
				let mut header = dev.header();
				let mut bars = header.base_addresses().iter().enumerate();
				while let Some((i, b)) = bars.next() {
					let (size, og) = b.size();

					let size = match size {
						Some(size) => size.get(),
						None => {
							assert_eq!(og, 0, "masked pci bar was not originally 0 (HW bug?)");
							continue;
						}
					};
					assert_ne!(size, u32::MAX, "TODO greater than 32 bit size (wow)");

					// Clear upper half if 64 bits.
					if pci::BaseAddress::is_64bit(og) {
						let (_, b) = bars.next().expect("bar can't be 64 bit");
						b.set(0);
					}

					// Set bar
					let size = usize::try_from(size).unwrap();
					let offt = mmio & (size - 1);
					if offt > 0 {
						mmio += size - offt;
					}
					kernel::sys_log!("mmio is 0x{:x}", mmio);
					b.set(u32::try_from(mmio).unwrap());

					// Push args
					let i = u128::try_from(i).unwrap();
					let a = u128::try_from(mmio).unwrap();
					let s = u128::try_from(size).unwrap();
					buf = match pci::BaseAddress::is_mmio(og) {
						true => {
							driver::BarMmio::new(i, a, s).to_args(buf, &mut alloc, &mut add_arg)
						}
						false => driver::BarIo::new(i, a, s).to_args(buf, &mut alloc, &mut add_arg),
					}
					.unwrap();

					mmio += size;
				}

				let ret = dux::task::spawn_elf(data, &mut [].iter().copied(), &args[..argc]);
				let address = ret.unwrap();
				kernel::sys_log!("Spawned driver as {}", address);
				unsafe {
					TASKS[TASKS_COUNT] = Task {
						address,
						child_address: child_address << 64,
					};
					TASKS_COUNT += 1;
				}
			} else {
				kernel::sys_log!("No driver found for {:x}|{:x}", v, d);
			}
		}
	}

	// Enable notifications / interrupts
	notification::init(&unique_irqs[..unique_irqs_count]);

	loop {
		const OP_OPEN: u8 = 128;

		let rx = dux::ipc::receive();
		match rx.opcode.map(|n| n.get()).unwrap_or(0) {
			OP_OPEN => unsafe {
				let intr = u128::from(rx.uuid);
				let task = TASKS[..TASKS_COUNT]
					.iter()
					.find(|t| usize::from(t.address) == rx.address)
					.unwrap();
				let mask_addr = task.child_address & INTERRUPT_MAP_MASK.child_address;
				let mask_intr = intr & INTERRUPT_MAP_MASK.child_interrupt;
				let intr = INTERRUPT_MAP[..INTERRUPT_MAP_COUNT.into()]
					.iter()
					.find(|intr| {
						intr.child_address == mask_addr && u128::from(intr.bus) == mask_intr
					})
					.unwrap();
				notification::add_interrupt_listener(intr.system, rx.address);
			},
			_ => (),
		}
	}
}
