//! Ranges of reserved but not allocated memory pages. Useful if an array may need to be very large.
//!
//! Since the kernel is small and not many things need a large, contiguous reserve of memory
//! addresses are hardcoded as needed.
//!
//! There are 3 types of mappings:
//!
//! * Global, kernel-only. Includes kernel code, memory allocators, ...
//! * Local, kernel-only. Includes process VMS, ...
//! * Local, userland. Includes only things accessible by userland.

use core::ptr::NonNull;

/// Structure used to denote a start and end range.
pub struct Range {
	/// The start address of a range.
	start: NonNull<u8>,
	/// The end address of a range (inclusive).
	end: NonNull<u8>,
}

/// Convienence macro for registering a range.
///
/// The total range begins at $start and finishes at $end. In-between ranges
/// are denoted by a size. $kernel denotes the size of the kernel and is
/// substracted from $end.
macro_rules! range {
	{
		[$start:literal, $end:literal, $kernel:literal]
	} => {
		pub const TOTAL_START: NonNull<u8> = NonNull::new($start).unwrap();
		pub const TOTAL_START: NonNull<u8> = NonNull::new($end - $kernel).unwrap();
	}
	{
		@offset $offset:ident
		[$start:literal, $end:literal, $kernel:literal]
		$n:ident => $s=expr,
		$($name:ident => $size:expr,)*
	} => {
		range!([$start, $end, $kernel])
		pub const $n: Range = Range {
			start: NonNull::new(($offset + $n) as *mut _).unwrap(),
			end: NonNull::new(($offset + $n + $s) as *mut _).unwrap(),
		};
	}
}

/// The maximum size needed for the page stack.
///
/// The stack size is determined with `MAX_TOTAL_MEMORY / PAGE_SIZE * POINTER_SIZE`, which can
/// also be expressed as `1 << (log2(MAX_TOTAL_MEMORY) - log2(PAGE_SIZE) + log2(POINTER_SIZE))`.
///
/// For 32-bit systems we need only `4G / 4K * 4 = 1M` memory at most normally. Special
/// cases include x86 PAE and RISC-V's 34 bit addresses but those aren't considered for
/// now.
///
/// For 64-bit systems we need only `16E / 4K * 8 = 32P` memory at most normally. This may seem
/// like a lot but remember it's all virtual and just reserved, not allocated.

#[cfg(target_pointer_width = "64")]
range! {
	[0xffff_ffff_ff00_0000, 0xffff_ffff_ffff_ffff, 65536]
	MEMORY_MANAGER => 
}
