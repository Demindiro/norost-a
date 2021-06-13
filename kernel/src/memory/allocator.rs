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

use super::{PPN, PPNRange, PPNBox};
use super::reserved::PMM_STACK;
use crate::arch;
use core::mem;
use core::slice;

/// Stacks of PPNs for fast allocation. The stack also act as a ring buffer when moving PPNs
/// to the tree.
pub(super) struct Stacks {
	/// The stacks
	stacks: &'static mut [[PPNBox; Self::STACK_SIZE as usize]],
	/// The top and base of each stack
	top_base: *mut (u16, u16),
}

/// An allocator including a bitmap and a stack.
pub struct Allocator {
	stacks: Stacks,
}


impl Stacks {
	/// The amount of PPNs per stack. Must be a power of `2`.
	const STACK_SIZE: u16 = 1 << 10;

	/// The amount of bytes used by a single stack, excluding top and bottom.
	const MEM_STACK_SIZE: usize = Self::STACK_SIZE as usize * mem::size_of::<PPN>();
	
	/// The amount of bytes used by a single stack, including top and bottom.
	pub(super) const MEM_TOTAL_SIZE: usize = Self::MEM_STACK_SIZE + mem::size_of::<(u16, u16)>();

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
		stack[index as usize] = ppn.into_raw();
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
		unsafe { Some(PPN::from_raw(ppn)) }
	}

	/// Pops a PPN from the bottom of the stack. Returns `None` if there are no entries left.
	///
	/// Note that PPNs at the bottom are older than higher PPNs and are less likely to be in
	/// cache. This makes them better candidates for insertion in the tree.
	#[must_use]
	#[allow(dead_code)]
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
		unsafe { Some(PPN::from_raw(ppn)) }
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
	pub fn new(pages: &mut [PPNRange]) -> Result<Self, ()> {
		// TODO zero out memory before handing it to the VMS.
		// Get minimum needed pages
		let hc = 1; // TODO
		let count = hc * Stacks::MEM_TOTAL_SIZE;
		let count = (count + arch::PAGE_MASK) & !arch::PAGE_MASK;
		let count = count / arch::PAGE_SIZE;
		let stacks = {
			let mut i = 0;
			arch::VirtualMemorySystem::allocate_pages(|| {
				loop {
					if let Some(p) = pages[i].pop() {
						break p;
					} else {
						i += 1;
					}
				}
			}, PMM_STACK.start.cast(), count as usize);
			PMM_STACK.start
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

		for p in pages {
			while let Some(p) = p.pop() {
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
