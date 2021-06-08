//! # Page-based memory allocator.
//!
//! I'm probably not the first to come up with this but I don't know what the "official" name is
//! hence I'm just calling it the "Allocator".
//!
//! ## Features
//!
//! * Hugepage support.
//!
//! * Reference counting.
//!
//! ## How it works.
//!
//! The allocator is split in a backend and a frontend.
//!
//! * The frontend is simply a stack with physical addresses (as PPNs). Popping and pushing onto it
//!   is very fast. Each hart has a separate stack to improve cache efficiency.
//!
//! * The backend is a tree structure that is very similar to VMA tables: The upper bits are the
//!   PPN of each page. The main difference is that the lower bits are used as a counter indicating
//!   how many pages are free. If the counter is 0, the table can be removed. If the counter is
//!   `PAGE_SIZE / PTE_SIZE`, the table can be merged into a hugepage.
//!
//! Using a VMS-like tree structure makes it trivial to support hugepages of any size.

use crate::arch::{self, PAGE_SIZE};
use core::mem;
use core::num::NonZeroUsize;
use core::ptr::NonNull;
use core::slice;

/// A single entry that is either a pointer to the next table or a PPN.
#[repr(transparent)]
struct Entry(usize);

/// A single table in the tree.
struct Table([Entry; Self::MAX]);

/// The lowest level of tables, which can only contain PPNs.
struct LowestTable([PPN; Table::MAX]);

/// The root of the tree.
struct Tree {
	root: &'static mut Table,
}

/// A struct representing a PPN.
///
/// A PPN **cannot** be directly used as a physical address. It is formatted such that it can
/// be put straight into a VMA table.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PPN(usize);

/// Stacks of PPNs for fast allocation. The stack also act as a ring buffer when moving PPNs
/// to the tree.
struct Stacks {
	/// The stacks
	stacks: &'static mut [[PPN; Self::STACK_SIZE as usize]],
	/// The top and base of each stack
	top_base: *mut (u16, u16),
}

/// An allocator including a tree and a stack.
pub struct Allocator {
	//tree: &'static mut Tree,
	stacks: Stacks,
}

impl Entry {
	/// The mask for the allocation counter.
	const COUNTER_MASK: usize = ((Table::MAX << 1) - 1);

	/// Returns this entry as a PPN if it is one.
	fn as_ppn<'a>(&'a mut self) -> Option<PPN> {
		if self.free_count() == Table::MAX {
			Some(PPN(self.0 & !Self::COUNTER_MASK))
		} else {
			None
		}
	}

	/// Returns this entry as a table if it is one. May return `None` if the
	/// table is full, in which case no table is allocated.
	fn as_table<'a>(&'a mut self) -> Option<&'a mut Table> {
		if self.free_count() != Table::MAX {
			unsafe { ((self.0 & !Self::COUNTER_MASK) as *mut Table).as_mut() }
		} else {
			None
		}
	}

	/// Returns this entry as a lowest level table if it is one. May return `None`
	/// if the table is full, in which case no table is allocated.
	fn as_lowest_table<'a>(&'a mut self) -> Option<&'a mut LowestTable> {
		if self.free_count() != Table::MAX {
			unsafe { ((self.0 & !Self::COUNTER_MASK) as *mut LowestTable).as_mut() }
		} else {
			None
		}
	}

	/// Returns the amount of free pages this PTE has.
	fn free_count(&self) -> usize {
		// 0..=512 can be represented with 10 bits, which we have.
		// 0..=1024 needs 11 bits, which we technically don't have.
		// However, since a leaf at the lowest level can't point to other levels, we actually have
		// up to 19 bits to use. A page is always aligned such that the lowest 12 bits are always
		// 0, we effectively have 12 bits for our use. This means that simply masking will always
		// give us a correct result.
		self.0 & Self::COUNTER_MASK
	}
}

impl Table {
	/// The maximum amount of PTEs per table. Always a power of 2.
	const MAX: usize = PAGE_SIZE / mem::size_of::<usize>();

	// /// Takes a PPN out.
}

impl<U> From<NonNull<U>> for PPN {
	fn from(ptr: NonNull<U>) -> Self {
		PPN(ptr.as_ptr() as usize >> 2)
	}
}

impl core::fmt::Debug for PPN {
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		write!(f, "PPN (page: 0x{:x})", self.0 << 2)
	}
}

impl Stacks {
	/// The amount of PPNs per stack. Must be a power of `2`.
	const STACK_SIZE: u16 = 1 << 10;

	/// The amount of bytes used by a single stack, excluding top and bottom.
	const MEM_STACK_SIZE: usize = Self::STACK_SIZE as usize * mem::size_of::<PPN>();
	
	/// The amount of bytes used by a single stack, including top and bottom.
	const MEM_TOTAL_SIZE: usize = Self::MEM_STACK_SIZE + mem::size_of::<(u16, u16)>();

	/// Pushes a PPN on the given stack. Returns `true` if successful, `false` if the
	/// stack is full.
	#[must_use]
	fn push(&mut self, stack_index: usize, ppn: PPN) -> bool {
		let stack = &mut self.stacks[stack_index];
		// SAFETY: the pointers point to arrays at least as large as stacks, and if the index
		// was OOB we'd have paniced already.
		let top_base = unsafe { &mut *self.top_base.add(stack_index) };
		if top_base.0 == top_base.1.wrapping_add(Self::STACK_SIZE) {
			// The stack is full.
			return false;
		}
		let index = top_base.0 & (Self::STACK_SIZE - 1);
		stack[index as usize] = ppn;
		top_base.0 = top_base.0.wrapping_add(1);
		true
	}

	/// Pops a PPN from the stack. Returns `None` if there are no entries left.
	#[must_use]
	fn pop(&mut self, stack_index: usize) -> Option<PPN> {
		let stack = &mut self.stacks[stack_index];
		// SAFETY: the pointers point to arrays at least as large as stacks, and if the index
		// was OOB we'd have paniced already.
		let top_base = unsafe { &mut *self.top_base.add(stack_index) };
		if top_base.0 == top_base.1 {
			// The stack is empty
			return None;
		}
		top_base.0 = top_base.0.wrapping_sub(1);
		let index = top_base.0 & (Self::STACK_SIZE - 1);
		let ppn = stack[index as usize];
		Some(ppn)
	}

	/// Pops a PPN from the bottom of the stack. Returns `None` if there are no entries left.
	///
	/// Note that PPNs at the bottom are older than higher PPNs and are less likely to be in
	/// cache. This makes them better candidates for insertion in the tree.
	#[must_use]
	fn pop_base(&mut self, stack_index: usize) -> Option<PPN> {
		let stack = &mut self.stacks[stack_index];
		// SAFETY: the pointers point to arrays at least as large as stacks, and if the index
		// was OOB we'd have paniced already.
		let top_base = unsafe { &mut *self.top_base.add(stack_index) };
		if top_base.0 == top_base.1 {
			// The stack is empty
			return None;
		}
		let index = top_base.1 & (Self::STACK_SIZE - 1);
		let ppn = stack[index as usize];
		top_base.1 = top_base.1.wrapping_add(1);
		Some(ppn)
	}
}

#[cfg(fals)]
impl Tree {
	/// The maximum depths of the tree.
	const LEVELS: usize = 3;

	/// Inserts a page into the tree.
	fn insert(&mut self, page: PPN) {
		
	}

	/// Remove and return a set of pages from the tree.
	/// It is a megapage except that some of the pages may already be allocated.
	fn batch_allocate(&mut self) -> Result<PPNBatch, ()> {
		let mut tbl = self.0;
		for _ in 0..LEVELS - 1 {
			let pte = tbl[0];
		}
	}
}

#[cfg(fals)]
impl Tree {
	// /// Inserts a megapage into the tree.
}

#[cfg(fals)]
impl Tree {
	// /// Inserts a gigapage into the tree.
}
	
#[cfg(fals)]
impl Tree {
	// /// Inserts a terapage into the tree.
}

impl Allocator {
	/// Creates a new `Allocator` with the given pages.
	pub fn new(pages: &[(PPN, usize)]) -> Result<Self, ()> {
		// Get minimum needed pages
		let hc = arch::hart_count();
		log!("hc: {}", hc);
		let count = hc * Stacks::MEM_TOTAL_SIZE;
		let count = (count + arch::PAGE_MASK) & !arch::PAGE_MASK;
		let count = count / arch::PAGE_SIZE;
		let (mut stacks, extra, pages) = 'a: loop {
			let mut c = 0;
			for (i, p) in pages.iter().enumerate() {
				c += p.1;
				if c > count {
					let m = c - p.1;
					let m = count - m;
					let m = (PPN(pages[i].0.0 + (m << 10)), pages[i].1 - m);
					break 'a (
						&pages[..i + 1],
						m,
						&pages[i + 1..],
					);
				}
			}
			return Err(());
		};
		let stacks = {
			let mut i = 0;
			super::vms::add_kernel_mapping(move || {
				let p = PPN(stacks[0].0.0 + (i << 10));
				i += 1;
				if i == stacks[0].1 {
					stacks = &stacks[1..];
				}
				p
			}, count, crate::arch::RWX::RW).cast::<u8>()
		};
		let stacks = unsafe {
			Stacks {
				stacks: slice::from_raw_parts_mut(stacks.cast().as_ptr(), hc),
				top_base: stacks.as_ptr().add(Stacks::MEM_STACK_SIZE * hc).cast()
			}
		};
		let mut s = Self {
			stacks,
		};

		for i in 0..extra.1 {
			let p = PPN(extra.0.0 + (i << 10));
			s.insert(p);
		}

		for p in pages {
			for i in 0..p.1 {
				let p = PPN(p.0.0 + (i << 10));
				s.insert(p);
			}
		}
 		
		Ok(s)
	}

	/// Allocate a page.
	pub fn alloc(&mut self) -> Result<PPN, ()> {
		// FIXME use hart IDs.
		Ok(self.stacks.pop(0).expect("TODO"))
	}
	
	/// Free a page.
	pub fn free(&mut self, page: PPN) {
		// FIXME use hart IDs.
		if !self.stacks.push(0, page) {
			todo!()
		}
	}

	/// Inserts an untracked page.
	pub fn insert(&mut self, page: PPN) {
		self.free(page)
	}
}

impl Drop for Allocator {
	fn drop(&mut self) {
		panic!("Allocator got dropped!");
	}
}
