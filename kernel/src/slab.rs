use crate::arch::{Page, VirtualMemorySystem};
use core::fmt;
use core::mem;
use core::ptr::NonNull;

/// An allocator with a large address range for personal use. It maps pages in as needed.
///
/// An allocation will either increase the bump counter of the allocator or it will take
/// a slab of free memory from the free list.
///
/// A deallocation will turn the memory into a linked list entry and insert it in front of
/// the free list.
///
/// A slab allocator cannot be safely dropped. Attempting to do so will result in a panic.
pub struct Slab<T> {
	/// The start of the address range.
	start: NonNull<T>,
	/// The maximum amount of entries that can be allocated.
	max: usize,
	/// The current amount of entries that can be allocated without mapping in more pages.
	capacity: usize,
	/// The bump counter, used for allocating fresh memory.
	bump_counter: usize,
	/// The start of the free list.
	free_list: Option<NonNull<Free>>,
}

/// A free memory entry
struct Free {
	/// The next free entry, if any.
	next: Option<NonNull<Free>>,
}

pub enum AllocateError {
	OutOfAddressSpace,
	AddError(AddError),
}

impl<T> Slab<T> {
	/// Initialize a new slab allocator. Can be done at compile time (i.e. `const`)
	///
	/// The `size` is in bytes.
	///
	/// ## Safety
	///
	/// The address range is not used by anything else. It must not overflow either.
	///
	/// ## Panics
	///
	/// T is a ZST.
	///
	/// T is larger than the size of a single page.
	pub const unsafe fn new(start: NonNull<u8>, size: usize) -> Self {
		const _ZST_CHECK: usize = mem::size_of::<T>() - 1;
		const _PAGE_CHECK: usize = Page::SIZE - mem::size_of::<T>();
		Self {
			start: start.cast(),
			max: size / mem::size_of::<T>(),
			capacity: 0,
			bump_counter: 0,
			free_list: None,
		}
	}

	/// Allocate memory.
	pub fn allocate(&mut self) -> Result<NonNull<T>, AllocateError> {
		if let Some(free) = self.free_list {
			self.free_list = unsafe { free.as_ref().next };
			Ok(free.cast())
		} else if self.bump_counter < self.capacity {
			self.bump_counter += 1;
			unsafe { Ok(NonNull::new_unchecked(self.start.add(self.bump_counter - 1))) }
		} else if self.capacity < self.max {
			let cur_size = (mem::size_of::<T>() * self.capacity + Page::MASK) & !Page::MASK;
			let ptr = unsafe {
				NonNull::new_unchecked(self.start.cast::<u8>().as_ptr().add(cur_size))
			}.cast();
			VirtualMemorySystem::allocate(ptr, 1, RWX::RW, false, true).map_err(AllocateError)?;
			let new_size = cur_size + Page::SIZE;
			self.capacity = new_size / mem::size_of::<T>();
			self.bump_counter
		} else {
			Err(AllocateError::OutOfAddressSpace)
		}
	}

	/// Deallocate memory.
	///
	/// ## Safety
	///
	/// The memory is not in use by anything. The pointer must also be one returned from `allocate`
	pub unsafe fn deallocate(&mut self, memory: NonNull<T>) {
		let next = self.free_list;
		mem.cast::<Free>().write(Free { next });
		self.free_list = Some(mem.cast());
	}
}

impl<T> Drop for Slab<T> {
	fn drop(&mut self) {
		panic!("Slab cannot be safely dropped");
	}
}

impl fmt::Debug for AllocateError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::OutOfAddressSpace => f.write_str("Out of address space"),
			Self::AddError(e) => write!("{}", e),
		}
	}
}
