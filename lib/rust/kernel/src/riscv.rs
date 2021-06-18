macro_rules! syscall {
	($name:ident, $code:literal) => {
		#[inline]
		pub unsafe fn $name() -> Return {
			let (status, value);
			asm!("ecall", in("a7") $code, lateout("a0") status, lateout("a1") value);
			Return { status, value }
		}
	};
	($name:ident, $code:literal, $a0:ident:$a0t:ty) => {
		#[inline]
		pub unsafe fn $name($a0: $a0t) -> Return {
			let (status, value);
			asm!("ecall", in("a7") $code, in("a0") $a0, lateout("a0") status, lateout("a1") value);
			Return { status, value }
		}
	};
	($name:ident, $code:literal, $a0:ident:$a0t:ty, $a1:ident:$a1t:ty) => {
		#[inline]
		pub unsafe fn $name($a0: $a0t, $a1: $a1t) -> Return {
			let (status, value);
			asm!("ecall", in("a7") $code, in("a0") $a0, in("a1") $a1, lateout("a0") status, lateout("a1") value);
			Return { status, value }
		}
	};
	($name:ident, $code:literal, $a0:ident:$a0t:ty, $a1:ident:$a1t:ty, $a2:ident:$a2t:ty) => {
		#[inline]
		pub unsafe fn $name($a0: $a0t, $a1: $a1t, $a2: $a2t) -> Return {
			let (status, value);
			asm!("ecall", in("a7") $code, in("a0") $a0, in("a1") $a1, in("a2") $a2, lateout("a0") status, lateout("a1") value);
			Return { status, value }
		}
	};
	($name:ident, $code:literal, $a0:ident:$a0t:ty, $a1:ident:$a1t:ty, $a2:ident:$a2t:ty, $a3:ident:$a3t:ty) => {
		#[inline]
		pub unsafe fn $name($a0: $a0t, $a1: $a1t, $a2: $a2t, $a3: $a3t) -> Return {
			let (status, value);
			asm!("ecall", in("a7") $code, in("a0") $a0, in("a1") $a1, in("a2") $a2, in("a3") $a3, lateout("a0") status, lateout("a1") value);
			Return { status, value }
		}
	};
}

#[repr(align(4096))]
pub struct Page {
	_data: [u8; Self::SIZE],
}

impl Page {
	pub const SIZE: usize = 4096;
	pub const MASK: usize = Self::SIZE - 1;
}
