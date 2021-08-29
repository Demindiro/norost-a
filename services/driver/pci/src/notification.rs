#[derive(Clone, Copy)]
struct Interrupt {
	tasks: [usize; 16],
	tasks_count: u8,
	interrupt: u16,
	/// Used to help ensure every task gets the right interrupt ASAP.
	///
	/// Probably doesn't work very well but WCYD
	index: u8,
}

static mut INTERRUPT_LISTENERS: [Interrupt; 16] = [Interrupt {
	tasks: [0; 16],
	tasks_count: 0,
	interrupt: 0,
	index: 0,
}; 16];
static mut INTERRUPT_LISTENERS_COUNT: u8 = 0;

#[naked]
extern "C" fn notification_handler_entry() {
	unsafe {
		asm!(
			"
			# a0: type
			# a1: value
			# a7: address
			#
			# The original a[0-2] are stored on the stack by the kernel.
			.equ	GP_REGBYTES, 8
			.equ	NOTIFY_RETURN, 9
			addi	sp, sp, -(13 + 4) * GP_REGBYTES
			sd		t0, 0 * GP_REGBYTES (sp)
			sd		t1, 1 * GP_REGBYTES (sp)
			sd		t2, 2 * GP_REGBYTES (sp)
			sd		t3, 3 * GP_REGBYTES (sp)
			sd		t4, 4 * GP_REGBYTES (sp)
			sd		t5, 5 * GP_REGBYTES (sp)
			sd		t6, 6 * GP_REGBYTES (sp)
			sd		a3, 7 * GP_REGBYTES (sp)
			sd		a4, 8 * GP_REGBYTES (sp)
			sd		a5, 9 * GP_REGBYTES (sp)
			sd		a6, 10 * GP_REGBYTES (sp)
			sd		a2, 11 * GP_REGBYTES (sp)
			sd		ra, 12 * GP_REGBYTES (sp)
			mv		a2, a7
			call	notification_handler
			ld		t0, 0 * GP_REGBYTES (sp)
			ld		t1, 1 * GP_REGBYTES (sp)
			ld		t2, 2 * GP_REGBYTES (sp)
			ld		t3, 3 * GP_REGBYTES (sp)
			ld		t4, 4 * GP_REGBYTES (sp)
			ld		t5, 5 * GP_REGBYTES (sp)
			ld		t6, 6 * GP_REGBYTES (sp)
			ld		a3, 7 * GP_REGBYTES (sp)
			ld		a4, 8 * GP_REGBYTES (sp)
			ld		a5, 9 * GP_REGBYTES (sp)
			ld		a6, 10 * GP_REGBYTES (sp)
			ld		a2, 11 * GP_REGBYTES (sp)
			ld		ra, 12 * GP_REGBYTES (sp)
			addi	sp, sp, (13 + 4) * GP_REGBYTES
			li		a7, NOTIFY_RETURN
			ecall
		",
			options(noreturn)
		);
	}
}

#[export_name = "notification_handler"]
extern "C" fn notification_handler(typ: usize, value: usize, address: usize) -> usize {
	if typ != 0 || address != usize::MAX {
		return usize::MAX;
	}
	let mut addr = usize::MAX;
	unsafe {
		INTERRUPT_LISTENERS
			.iter_mut()
			.find(|e| usize::from(e.interrupt) == value)
			.map(|e| {
				addr = e.tasks[usize::from(e.index)];
				e.index += 1;
				e.index %= e.tasks_count;
			})
			.or_else(|| Some(kernel::sys_log!("Someone's naughty on IRQ 0x{:x}", value)));
	}
	addr
}

pub(crate) fn init(irqs: &[u16]) {
	let ret = unsafe { kernel::io_set_notify_handler(notification_handler_entry) };
	assert_eq!(ret.status, 0, "failed to set notify handler");

	for irq in irqs.iter().copied() {
		loop {
			let ret = unsafe { kernel::sys_reserve_interrupt(irq.into()) };
			match ret.status {
				0 => break,
				11 => panic!("interrupt already reserved"),
				12 => continue,
				_ => panic!("failed to reserve interrupt: {}", ret.status),
			}
		}
	}
}

pub(crate) fn add_interrupt_listener(interrupt: u16, address: usize) {
	unsafe {
		match INTERRUPT_LISTENERS
			.iter_mut()
			.find(|e| e.interrupt == interrupt)
		{
			Some(e) => {
				e.tasks[usize::from(e.tasks_count)] = address;
				e.tasks_count += 1;
			}
			None => {
				INTERRUPT_LISTENERS[usize::from(INTERRUPT_LISTENERS_COUNT)] = Interrupt {
					tasks: [0; 16],
					tasks_count: 1,
					interrupt,
					index: 0,
				};
				INTERRUPT_LISTENERS[usize::from(INTERRUPT_LISTENERS_COUNT)].tasks[0] = address;
				INTERRUPT_LISTENERS_COUNT += 1;
			}
		}
	}
}
