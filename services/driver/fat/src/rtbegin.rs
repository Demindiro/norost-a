use core::mem;
use core::slice;

#[export_name = "__arg_count"]
static mut ARG_COUNT: usize = 0;
#[export_name = "__arg_ptr"]
static mut ARG_POINTER: *const *const u8 = core::ptr::null();

pub fn args() -> ArgIter {
	let ptr = unsafe { ARG_POINTER };
	let end = unsafe { ptr.add(ARG_COUNT) };
	ArgIter { ptr, end }
}

pub struct ArgIter {
	ptr: *const *const u8,
	end: *const *const u8,
}

impl Iterator for ArgIter {
	type Item = &'static [u8];

	fn next(&mut self) -> Option<Self::Item> {
		(self.ptr != self.end).then(|| unsafe {
			let len = usize::from(*(*self.ptr).cast::<u16>());
			let ret = slice::from_raw_parts((*self.ptr).add(mem::size_of::<u16>()), len);
			self.ptr = self.ptr.add(1);
			ret
		})
	}
}

global_asm!(
	"
	.globl	_start
	_start:
		# Take note of arguments and argument count
		ld		t0, -8(sp)
		addi	sp, sp, -8
		slli	t1, t0, 3
		sub		sp, sp, t1
		lla		t2, __arg_count
		lla		t3, __arg_ptr
		sd		t0, 0(t2)
		sd		sp, 0(t3)

		# Set return address to 0 to aid debugger
		sd		zero, -8(sp)
		addi	sp, sp, -8

		call	main

		# Loop forever as we can't exit
	0:
		j		0b

	66:	# Abort (TODO)
		j		66b
	",
);
