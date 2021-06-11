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
//!
//!
//! ## The maximum size needed for the allocation bitmap.
//!
//! The bitmap size is determined with `MAX_TOTAL_MEMORY / PAGE_SIZE / 8`, which can also be
//! expressed as `1 << (log2(MAX_TOTAL_MEMORY) - log2(PAGE_SIZE) - 3)`.
//!
//! For 32-bit systems we need only `4G / 4K / 8 = 32K` memory at most normally. Special
//! cases include RISC-V's 34 bit addresses, which simply needs `128K`, and x86's PAE with
//! 36-bit addresses, which needs `512K`.
//!
//! For 64-bit systems we need only `16E / 4K / 8 = 512T` memory at most normally. This may seem
//! like a lot but remember it's all virtual and just reserved, not allocated. Still, it is outside
//! the range of VMSes like Sv39, which can map at most `512G` of memory.
//!
//! Since it's unlikely that a system with only `Sv39` will have more than `512T` of memory we can
//! instead reserve only `512T / 4K / 8 = 512M`, which is well within addressable range.
//!
//! The `512T` limit is chosen as it allows 32-bit PPNs (`44 - 12 = 32`)
//!
//! 
//! ## The maximum size needed for the allocation stacks
//!
//! The amount of memory needed is ``CPU_CORES * STACK_SIZE``. In theory, the amount of memory
//! needed is larger than the available virtual and physical memory but in practice the amount
//! of CPU cores is very limited. For now, `4096` is assumed to be the practical limit for
//! commercial CPUs in the future.


use core::ptr::NonNull;

/// Structure used to denote a start and end range.
pub struct Range {
	/// The start address of a range.
	pub start: NonNull<u8>,
	/// The end address of a range (inclusive).
	pub end: NonNull<u8>,
}

/// Convienence macro for registering a range.
///
/// The total range begins at $start and finishes at $end. In-between ranges
/// are denoted by a size. $kernel denotes the size of the kernel and is
/// substracted from $end.
#[allow(unused)]
macro_rules! range {
	// LOCAL
	{
		@offset $offset:expr,
		@local $g_offset:expr,
	} => {
		pub const LOCAL: Range = Range {
			end: unsafe { NonNull::new_unchecked($g_offset.wrapping_sub(1) as *mut _) },
			start: unsafe { NonNull::new_unchecked(($offset & !MAX_PAGE_MASK) as *mut _) },
		};
	};
	{
		@offset $offset:expr,
		@local $g_offset:expr,
		$n:ident => $s:expr,
		$($l_name:ident => $l_size:expr,)*
	} => {
		pub const $n: Range = Range {
			end: unsafe { NonNull::new_unchecked($offset.wrapping_sub(1) as *mut _) },
			start: unsafe { NonNull::new_unchecked($offset.wrapping_sub($s) as *mut _) },
		};
		range! {
			@offset $offset.wrapping_sub($s),
			@local $g_offset,
			$($l_name => $l_size,)*
		}
	};
	// GLOBAL
	{
		@offset $offset:expr,
		@global
		@local
		$($l_name:ident => $l_size:expr,)*
	} => {
		pub const GLOBAL: Range = Range {
			end: unsafe { NonNull::new_unchecked(0usize.wrapping_sub(1) as *mut _) },
			start: unsafe { NonNull::new_unchecked(($offset & !MAX_PAGE_MASK) as *mut _) },
		};
		range! {
			@offset $offset & !MAX_PAGE_MASK,
			@local $offset & !MAX_PAGE_MASK,
			$($l_name => $l_size,)*
		}
	};
	{
		@offset $offset:expr,
		@global
		$n:ident => $s:expr,
		$($g_name:ident => $g_size:expr,)*
		@local
		$($l_name:ident => $l_size:expr,)*
	} => {
		pub const $n: Range = Range {
			end: unsafe { NonNull::new_unchecked($offset.wrapping_sub(1) as *mut _) },
			start: unsafe { NonNull::new_unchecked($offset.wrapping_sub($s) as *mut _) },
		};
		range! {
			@offset $offset.wrapping_sub($s),
			@global
			$($g_name => $g_size,)*
			@local
			$($l_name => $l_size,)*
		}
	};
	// DUMP
	[@dump $total:ident] => {
		log!("    {:<16}{:p}-{:p}", stringify!($total), $total.start, $total.end);
	};
	[@dump $total:ident $n:ident, $($name:ident,)*] => {
		log!("    {:<16}{:p}-{:p}", stringify!($n), $n.start, $n.end);
		range![@dump $total $($name,)*];
	};
	// PUB
	{
		limit = $limit:expr,
		[GLOBAL]
		$($g_name:ident => $g_size:expr,)*
		[LOCAL]
		$($l_name:ident => $l_size:expr,)*
	} => {
		range! {
			@offset 0usize,
			@global
			$($g_name => $g_size,)*
			@local
			$($l_name => $l_size,)*
		}

		pub fn dump_vms_map() {
			log!("Virtual memory map:");
			log!("  Global:");
			range![@dump GLOBAL $($g_name,)*];
			log!("  Local:");
			range![@dump LOCAL $($l_name,)*];
		}

		const _BORDER_CHECK: usize = unsafe { (LOCAL.start.as_ptr() as usize) } - $limit;
	};
}

const MAX_HARTS: usize = 4096;
const MAX_PAGE_SIZE: usize = 1 << 30;
const MAX_PAGE_MASK: usize = MAX_PAGE_SIZE - 1;

#[cfg(target_arch = "riscv32")]
range! {
	KERNEL => 1 << 16,
	MEMORY_MANAGER => 1 << (30 - 12 - 3),
}

// Configuration for riscv64 with Sv39 VMS
#[cfg(target_arch = "riscv64")]
range! {
	limit = 0xffff_ff80_0000_0000,
	[GLOBAL]
	KERNEL => 1 << 16,
	PMM_BITMAP => 1 << (44 - 12 - 3),
	PMM_STACK => super::allocator::Stacks::MEM_TOTAL_SIZE * MAX_HARTS,
	SHARED_COUNTERS => 1 << (44 - 12 + 2),
	SHARED_ALLOC => 1 << (44 - 12 - 12 + 1),
	HART_STACKS => (1 << 12) * MAX_HARTS * 2, // Reserve extra space for guard pages.
	[LOCAL]
	HIGHMEM_A => 1 << 30,
	HIGHMEM_B => 1 << 30,
	VMM_ROOT => 1 << 12,
	TASK_DATA => 1 << 12,
}
