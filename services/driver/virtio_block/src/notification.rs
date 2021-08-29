#[naked]
extern "C" fn notification_handler() {
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
			li		a0, -1
			li		a7, NOTIFY_RETURN
			ecall
		",
			options(noreturn)
		);
	}
}

pub(crate) fn init() {
	let ret = unsafe { kernel::io_set_notify_handler(notification_handler) };
	assert_eq!(ret.status, 0, "failed to set notify handler");
}
