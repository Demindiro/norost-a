macro_rules! syscall {
	($name:ident, $code:literal) => {
		#[inline(always)]
		#[must_use]
		pub unsafe fn $name() -> Return {
			let (status, value);
			asm!("ecall", in("a7") $code, lateout("a0") status, lateout("a1") value);
			Return { status, value }
		}
	};
	($name:ident, $code:literal, $a0:ident:$a0t:ty) => {
		#[inline(always)]
		#[must_use]
		pub unsafe fn $name($a0: $a0t) -> Return {
			let (status, value);
			asm!("ecall", in("a7") $code, in("a0") $a0, lateout("a0") status, lateout("a1") value);
			Return { status, value }
		}
	};
	($name:ident, $code:literal, $a0:ident:$a0t:ty, $a1:ident:$a1t:ty) => {
		#[inline(always)]
		#[must_use]
		pub unsafe fn $name($a0: $a0t, $a1: $a1t) -> Return {
			let (status, value);
			asm!("ecall", in("a7") $code, in("a0") $a0, in("a1") $a1, lateout("a0") status, lateout("a1") value);
			Return { status, value }
		}
	};
	($name:ident, $code:literal, $a0:ident:$a0t:ty, $a1:ident:$a1t:ty, $a2:ident:$a2t:ty) => {
		#[inline(always)]
		#[must_use]
		pub unsafe fn $name($a0: $a0t, $a1: $a1t, $a2: $a2t) -> Return {
			let (status, value);
			asm!("ecall", in("a7") $code, in("a0") $a0, in("a1") $a1, in("a2") $a2, lateout("a0") status, lateout("a1") value);
			Return { status, value }
		}
	};
	($name:ident, $code:literal, $a0:ident:$a0t:ty, $a1:ident:$a1t:ty, $a2:ident:$a2t:ty, $a3:ident:$a3t:ty) => {
		#[inline(always)]
		#[must_use]
		pub unsafe fn $name($a0: $a0t, $a1: $a1t, $a2: $a2t, $a3: $a3t) -> Return {
			let (status, value);
			asm!("ecall", in("a7") $code, in("a0") $a0, in("a1") $a1, in("a2") $a2, in("a3") $a3, lateout("a0") status, lateout("a1") value);
			Return { status, value }
		}
	};
	($name:ident, $code:literal, $a0:ident:$a0t:ty, $a1:ident:$a1t:ty, $a2:ident:$a2t:ty, $a3:ident:$a3t:ty, $a4:ident:$a4t:ty) => {
		#[inline(always)]
		#[must_use]
		pub unsafe fn $name($a0: $a0t, $a1: $a1t, $a2: $a2t, $a3: $a3t, $a4: $a4t) -> Return {
			let (status, value);
			asm!("ecall", in("a7") $code, in("a0") $a0, in("a1") $a1, in("a2") $a2, in("a3") $a3, in("a4") $a4, lateout("a0") status, lateout("a1") value);
			Return { status, value }
		}
	};
	($name:ident, $code:literal, $a0:ident:$a0t:ty, $a1:ident:$a1t:ty, $a2:ident:$a2t:ty, $a3:ident:$a3t:ty, $a4:ident:$a4t:ty, $a5:ident:$a5t:ty) => {
		#[inline(always)]
		#[must_use]
		pub unsafe fn $name($a0: $a0t, $a1: $a1t, $a2: $a2t, $a3: $a3t, $a4: $a4t, $a5: $a5t) -> Return {
			let (status, value);
			asm!("ecall", in("a7") $code, in("a0") $a0, in("a1") $a1, in("a2") $a2, in("a3") $a3, in("a4") $a4, in("a5") $a5, lateout("a0") status, lateout("a1") value);
			Return { status, value }
		}
	};
	(saveall $name:ident, $code:literal, $a0:ident:$a0t:ty) => {
		#[inline(always)]
		pub unsafe fn $name($a0: $a0t) {
			asm!("ecall", in("a7") $code, in("a0") $a0);
		}
	};
}

/// Representation of a single memory page.
#[repr(align(4096))]
#[repr(C)]
pub struct Page([u8; Self::SIZE]);

impl Page {
	pub const OFFSET_BITS: u8 = 12;
	pub const SIZE: usize = 1 << Self::OFFSET_BITS;
	pub const ALIGN: usize = Self::SIZE;
	pub const MASK: usize = Self::SIZE - 1;
}
