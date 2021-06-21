//! Implementation of SV39 Virtual Memory System
//!
//! ## References
//!
//! [RISC-V Priviliged Architecture][rv], chapter 4.4
//! "Sv39: Page-Based 39-bit Virtual-Memory System"
//!
//! [rv]: https://riscv.org/wp-content/uploads/2017/05/riscv-privileged-v1.10.pdf

use super::{AddError, RWX};
use crate::arch;
use crate::arch::{Page, Map, MapRange};
use crate::memory::{self, AllocateError, PPN, PPNBox, PPNRange, PPNDirect, SharedPPN};
use crate::memory::reserved::{self, GLOBAL, VMM_ROOT};
use core::convert::{TryInto, TryFrom};
use core::mem;
use core::ops;
use core::ptr::NonNull;

/// The start index of the global kernel table.
const GLOBAL_KERNEL_TABLE_START_INDEX: usize = unsafe {
	(GLOBAL.start.as_ptr() as usize >> 30) & 0x1ff
};

/// The root table (level 2).
const ROOT: NonNull<[Entry; 512]> = VMM_ROOT.start.cast();

/// HIGHMEM_A
const HIGHMEM_A: NonNull<Page> = reserved::HIGHMEM_A.start.cast();

/// HIGHMEM_B
const HIGHMEM_B: NonNull<Page> = reserved::HIGHMEM_B.start.cast();

/// Page table entry
///
/// The format from MSb to LSb is:
///
/// - 10 bits reserved
/// - 26 bits PPN[2]
/// - 9 bits PPN[1]
/// - 9 bits PPN[0]
/// - 2 bits RSW, free for us to use
///   - 0b00 indicates a private mapping
///   - 0b01 indicates a direct mapping
///   - 0b10 indicates a shared mapping
///   - 0b11 indicates a shared but locked mapping
/// - 1 bit Dirty flag
/// - 1 bit Accessed flag
/// - 1 bit Global flag
/// - 1 bit Usermode flag
/// - 1 bit eXecute flag
/// - 1 bit Write flag
/// - 1 bit Read flag
/// - 1 bit Valid flag
#[repr(transparent)]
struct Entry(u64);

#[repr(transparent)]
struct Leaf(u64);

/// Page table.
///
/// Each table contains 512 entries.
#[repr(align(4096))]
struct Table([Entry; 512]);

/// Page-allocated page table.
struct TablePage(NonNull<Page>);

struct VirtualAddress(u64);

/// The root table of a Sv39 VMS.
#[repr(C)]
pub struct Sv39(u64);

impl Entry {
	const VALID_MASK: u64 = 0b1;

	const RWX_MASK: u64 = 0b1110;

	const USERMODE_MASK: u64 = 0b1_0000;

	const GLOBAL_BIT: u8 = 0b10_0000;

	/// Create a new entry for an invalid entry.
	fn new_invalid() -> Self {
		Self(0)
	}

	/// Create a new entry for a single physical entry.
	#[must_use]
	fn new_leaf2(ppn: usize, rwx: RWX, usermode: bool, global: bool) -> Self {
		let mut s = Self((ppn as u64) >> 2);
		s.set_rwx(rwx);
		s.set_valid(true);
		s.set_usermode(usermode);
		s.set_global(global);
		s.0 |= 0b1100_0000;
		s
	}

	/// Create a new entry for a single table entry.
	#[must_use]
	fn new_table(ppn: PPN) -> Self {
		let ppn = ppn.into_raw();
		let mut s = Self((ppn as u64) << 10);
		s.set_valid(true);
		s
	}

	/// Return whether this entry is valid.
	#[must_use]
	fn is_valid(&self) -> bool {
		self.0 & Self::VALID_MASK > 0
	}

	/// Set whether this entry is valid.
	fn set_valid(&mut self, valid: bool) {
		self.0 &= !Self::VALID_MASK;
		self.0 |= u64::from(valid) * Self::VALID_MASK;
	}

	/// Return the RWX flags or `None` if it is not a leaf entry.
	#[must_use]
	fn rwx(&self) -> Option<RWX> {
		RWX::try_from(self.0 & Self::RWX_MASK).ok()
	}

	/// Set the RWX flags.
	fn set_rwx(&mut self, rwx: RWX) {
		self.0 &= !Self::RWX_MASK;
		self.0 |= u64::from(rwx);
	}

	/// Set whether this page can be accesses by usermode.
	fn set_usermode(&mut self, allow: bool) {
		self.0 &= !Self::USERMODE_MASK;
		self.0 |= u64::from(allow) * Self::USERMODE_MASK;
	}

	fn set_global(&mut self, global: bool) {
		self.0 &= !(1 << Self::GLOBAL_BIT);
		self.0 |= u64::from(global) << Self::GLOBAL_BIT;
	}

	#[must_use]
	fn is_table(&self) -> bool {
		self.rwx().is_none()
	}
}

#[derive(Debug)]
pub enum PrivateOrShared {
	Private(PPN),
	Shared(SharedPPN),
}

impl Leaf {
	const VALID_BIT: u64 = 0;
	const USERMODE_BIT: u64 = 4;
	const GLOBAL_BIT: u64 = 5;
	const ACCESSED_BIT: u64 = 6;
	const DIRTY_BIT: u64 = 7;
	const TYPE_MASK: u64 = 0b11 << 8;
	const TYPE_PRIVATE: u64 = 0b00 << 8;
	const TYPE_DIRECT: u64 = 0b01 << 8;
	const TYPE_SHARED: u64 = 0b10 << 8;
	const TYPE_SHARED_LOCKED: u64 = 0b11 << 8;

	fn clear(&mut self) -> Result<PrivateOrShared, ()> {
		if self.is_valid() {
			let ppn = unsafe { PPN::from_raw((self.0 >> 10) as u32) };
			self.0 = 0;
			if self.is_shared() {
				Ok(PrivateOrShared::Shared(unsafe { SharedPPN::from_raw(ppn) }))
			} else {
				Ok(PrivateOrShared::Private(ppn))
			}
		} else {
			Err(())
		}
	}

	fn set(&mut self, map: Map, rwx: RWX, usermode: bool, global: bool) -> Result<(), AddError> {
		if !self.is_valid() {
			self.0 = 0;
			let ppn = match map {
				Map::Private(ppn) => {
					self.0 |= Self::TYPE_PRIVATE;
					ppn.into_raw()
				}
				Map::Direct(ppn) => {
					self.0 |= Self::TYPE_DIRECT;
					ppn.into()
				}
				Map::Shared(ppn) => {
					self.0 |= Self::TYPE_SHARED;
					ppn.into_raw().into_raw()
				}
				Map::SharedLocked(ppn) => {
					self.0 |= Self::TYPE_SHARED_LOCKED;
					ppn.into_raw().into_raw()
				}
			};
			self.0 |= u64::from(ppn) << 10;
			self.0 |= 1 << Self::VALID_BIT;
			self.0 |= u64::from(rwx);
			self.0 |= (usermode as u64) << Self::USERMODE_BIT;
			self.0 |= (global as u64) << Self::GLOBAL_BIT;
			self.0 |= 1 << Self::ACCESSED_BIT;
			self.0 |= 1 << Self::DIRTY_BIT;
			Ok(())
		} else {
			Err(AddError::Overlaps)
		}
	}

	fn share(&self) -> Result<Map, ()> {
		if self.is_valid() {
			if self.0 & Self::TYPE_MASK == Self::TYPE_DIRECT {
				Ok(Map::Direct(PPNDirect::from((self.0 >> 10) as u32)))
			} else {
				let ppn = unsafe { PPN::from_raw((self.0 >> 10) as u32) };
				if true || self.is_shared() {
					Ok(Map::Shared(unsafe { SharedPPN::from_raw(ppn) }))
				} else {
					Ok(Map::Shared(SharedPPN::new(ppn).expect("TODO")))
				}
			}
		} else {
			Err(())
		}
	}

	#[must_use]
	fn is_valid(&self) -> bool {
		self.0 & (1 << Self::VALID_BIT) > 0
	}

	#[must_use]
	fn is_shared(&self) -> bool {
		self.0 & Self::TYPE_SHARED > 0 || self.0 & Self::TYPE_SHARED_LOCKED > 0
	}
}

impl ops::Index<u64> for Table {
	type Output = Entry;

	fn index(&self, index: u64) -> &Self::Output {
		&self.0[index as usize]
	}
}

impl ops::IndexMut<u64> for Table {
	fn index_mut(&mut self, index: u64) -> &mut Self::Output {
		&mut self.0[index as usize]
	}
}

impl ops::Deref for Table {
	type Target = [Entry; 512];

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl ops::DerefMut for Table {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl ops::Deref for TablePage {
	type Target = Table;

	fn deref(&self) -> &Self::Target {
		// SAFETY: We own a unique reference to a valid page. The entries
		// in the page are all valid.
		unsafe { self.0.cast().as_ref() }
	}
}

impl ops::DerefMut for TablePage {
	fn deref_mut(&mut self) -> &mut Self::Target {
		// SAFETY: We own a unique reference to a valid page. The entries
		// in the page are all valid.
		unsafe { self.0.cast().as_mut() }
	}
}

impl VirtualAddress {
	const PPN_2_OFFSET: u64 = 30;
	const PPN_1_OFFSET: u64 = 21;
	const PPN_0_OFFSET: u64 = 12;

	const PPN_2_MASK: usize = 0x1ff << Self::PPN_2_OFFSET;
	const PPN_1_MASK: usize = 0x1ff << Self::PPN_1_OFFSET;
	const PPN_0_MASK: usize = 0x1ff << Self::PPN_0_OFFSET;

	/// Return the PPN[2] shifted to the right.
	fn ppn_2(&self) -> usize {
		(self.0 as usize & Self::PPN_2_MASK) >> Self::PPN_2_OFFSET
	}

	/// Return the PPN[1] shifted to the right.
	fn ppn_1(&self) -> usize {
		(self.0 as usize & Self::PPN_1_MASK) >> Self::PPN_1_OFFSET
	}

	/// Return the PPN[0] shifted to the right.
	fn ppn_0(&self) -> usize {
		(self.0 as usize & Self::PPN_0_MASK) >> Self::PPN_0_OFFSET
	}
}

impl Sv39 {

	/// Create a new Sv39 mapping.
	#[allow(dead_code)]
	pub fn new() -> Result<Self, AllocateError> {
		// Allocate 3 pages to map to ROOT.
		let ppn_2 = memory::allocate()?;
		let ppn_1 = memory::allocate()?;
		let ppn_0 = memory::allocate()?;

		let va = VirtualAddress(ROOT.as_ptr() as u64);

		let satp = ((ppn_2.as_usize() as u64) >> 12) & (1 << 63);

		// Map global kernel PTEs.
		unsafe {
			let curr = ROOT.cast::<[u64; 512]>().as_mut();
			Self::map_highmem_a(Some(&ppn_1));
			let new = Self::translate_highmem_a(&ppn_1).cast::<[u64; 512]>().as_mut();
			for i in GLOBAL_KERNEL_TABLE_START_INDEX..512 {
				new[i] = curr[i];
			}
		}

		// Map ROOT
		unsafe {
			let ppn_2 = ppn_2.into_raw();
			let ppn_2_alias = PPN::from_raw(ppn_2);
			let ppn_2 = PPN::from_raw(ppn_2);

			// Add a PTE pointing to VPN[2] in VPN[0]
			Self::map_highmem_a(Some(&ppn_0));
			Self::flush_highmem_a();
			Self::translate_highmem_a(&ppn_0).cast::<[Leaf; 512]>().as_mut()[va.ppn_0()]
				.set(Map::Private(ppn_2_alias), RWX::RW, false, false).unwrap();

			// Add a PTE pointing to VPN[0] in VPN[1]
			Self::map_highmem_a(Some(&ppn_1));
			Self::flush_highmem_a();
			Self::translate_highmem_a(&ppn_1).cast::<[Entry; 512]>().as_mut()[va.ppn_0()]
				= Entry::new_table(ppn_0);

			// Add a PTE pointing to VPN[1] in VPN[2]
			Self::map_highmem_a(Some(&ppn_2));
			Self::flush_highmem_a();
			Self::translate_highmem_a(&ppn_2).cast::<[Entry; 512]>().as_mut()[va.ppn_0()]
				= Entry::new_table(ppn_1);
		}

		// Unmap HIGHMEM_A
		unsafe {
			Self::map_highmem_a(None);
			Self::flush_highmem_a();
		}

		Ok(Self(satp))
	}

	/// Uses HIGHMEM_A
	fn get_pte(address: NonNull<Page>) -> Result<NonNull<Leaf>, AddError> {
		let va = VirtualAddress(address.as_ptr() as u64);

		// VPN[2]
		let pte = &unsafe { ROOT.as_ref() }[va.ppn_2()];
		if !pte.is_table() {
			return todo!();
		}

		// VPN[1]
		let ppn = unsafe { PPN::from_raw((pte.0 >> 10) as u32) };
		unsafe { Self::map_highmem_a(Some(&ppn)) };
		Self::flush_highmem_a();
		let tbl = unsafe { Self::translate_highmem_a(&ppn).cast::<[Entry; 512]>() };
		let tbl = &mut unsafe { &mut *tbl.as_ptr() };
		let pte = &mut tbl[va.ppn_1()];
		if !pte.is_table() {
			return todo!();
		}

		// VPN[0]
		let ppn = unsafe { PPN::from_raw((pte.0 >> 10) as u32) };
		unsafe { Self::map_highmem_a(Some(&ppn)) };
		Self::flush_highmem_a();
		let tbl = unsafe { Self::translate_highmem_a(&ppn).cast::<[Leaf; 512]>() };
		let tbl = &mut unsafe { &mut *tbl.as_ptr() };
		let pte = &mut tbl[va.ppn_0()];
		Ok(NonNull::from(pte))
	}

	/// Uses HIGHMEM_B
	fn get_pte_alloc(address: NonNull<Page>) -> Result<NonNull<Leaf>, AddError> {
		let va = VirtualAddress(address.as_ptr() as u64);

		// PPN[2]
		let pte = &mut unsafe { &mut *ROOT.as_ptr() }[va.ppn_2()];
		if !pte.is_valid() {
			let ppn = memory::allocate().map_err(AddError::AllocateError)?;
			*pte = Entry::new_table(ppn);
		} else if !pte.is_table() {
			return Err(AddError::Overlaps);
		}

		// PPN[1]
		let ppn = unsafe { PPN::from_raw((pte.0 >> 10) as u32) };
		unsafe { Self::map_highmem_b(Some(&ppn)) };
		Self::flush_highmem_b();
		let tbl = unsafe { Self::translate_highmem_b(&ppn).cast::<[Entry; 512]>() };
		let tbl = &mut unsafe { &mut *tbl.as_ptr() };
		let pte = &mut tbl[va.ppn_1()];
		if !pte.is_valid() {
			let ppn = memory::allocate().map_err(AddError::AllocateError)?;
			*pte = Entry::new_table(ppn);
		} else if !pte.is_table() {
			return Err(AddError::Overlaps);
		}

		// PPN[0]
		let ppn = unsafe { PPN::from_raw((pte.0 >> 10) as u32) };
		unsafe { Self::map_highmem_b(Some(&ppn)) };
		Self::flush_highmem_b();
		let tbl = unsafe { Self::translate_highmem_b(&ppn).cast::<[Leaf; 512]>() };
		let tbl = &mut unsafe { &mut *tbl.as_ptr() };
		let pte = &mut tbl[va.ppn_0()];
		Ok(NonNull::from(pte))
	}

	/// Uses HIGHMEM_B
	fn get_pte_alloc_mega(address: NonNull<Page>) -> Result<NonNull<Leaf>, AddError> {
		let va = VirtualAddress(address.as_ptr() as u64);

		// PPN[2]
		let pte = &mut unsafe { &mut *ROOT.as_ptr() }[va.ppn_2()];
		if !pte.is_valid() {
			let ppn = memory::allocate().map_err(AddError::AllocateError)?;
			*pte = Entry::new_table(ppn);
		} else if !pte.is_table() {
			return Err(AddError::Overlaps);
		}

		// PPN[1]
		let ppn = unsafe { PPN::from_raw((pte.0 >> 10) as u32) };
		unsafe { Self::map_highmem_b(Some(&ppn)) };
		Self::flush_highmem_b();
		let tbl = unsafe { Self::translate_highmem_b(&ppn).cast::<[Leaf; 512]>() };
		let tbl = &mut unsafe { &mut *tbl.as_ptr() };
		let pte = &mut tbl[va.ppn_1()];
		Ok(NonNull::from(pte))
	}

	/// Uses HIGHMEM_B
	fn get_pte_alloc_giga(address: NonNull<Page>) -> Result<NonNull<Leaf>, AddError> {
		let va = VirtualAddress(address.as_ptr() as u64);

		// PPN[2]
		let tbl = &mut unsafe { &mut *ROOT.cast::<[Leaf; 512]>().as_ptr() };
		let pte = &mut tbl[va.ppn_2()];
		Ok(NonNull::from(pte))
	}

	/// Allocate the given amount of private pages and insert it as virtual memory at the
	/// given address.
	pub fn allocate(&mut self, virtual_address: NonNull<Page>, count: usize, rwx: RWX, usermode: bool, global: bool) -> Result<(), AddError> {
		   let mut va = virtual_address;
		   // FIXME deallocate pages on failure.
		   memory::mem_allocate_range(count, |ppn| {
			   Self::add(va, Map::Private(ppn), rwx, true, false).unwrap();
			   va = NonNull::new(va.as_ptr().wrapping_add(1)).unwrap();
		   }).unwrap();
		   Ok(())
	}

	/// Add a single page mapping.
	pub fn add(address: NonNull<Page>, map: Map, rwx: RWX, usermode: bool, global: bool) -> Result<(), AddError> {
		let mut pte = Self::get_pte_alloc(address)?;
		unsafe {
			pte.as_mut().set(map, rwx, usermode, global).map_err(|_| AddError::Overlaps)
		}
	}

	/// Map a range of pages. If the range of pages as well as the address are well aligned mega-
	/// and/or gigapages will be used.
	pub fn add_range(
		mut address: NonNull<Page>,
		mut map_range: MapRange,
		rwx: RWX,
		usermode: bool,
		global: bool
	) -> Result<(), AddError> {

		let count = map_range.len();
		let ppn_min = PPNBox::try_from(map_range.start()).unwrap();
		let ppn_max = ppn_min.checked_add(count.try_into().unwrap()).ok_or(AddError::OutOfRange)?;

		let addr = address.as_ptr() as usize;
		if addr % (1 << 30) == 0
			&& PPNBox::from(ppn_min) % (1 << 18) == 0
			&& PPNBox::from(ppn_max) % (1 << 18) == 0
		{
			let mut undo = #[cold] |err: AddError| todo!();
			while let Some(map) = map_range.pop_base() {
				let c = map_range.forget_base((1 << 18) - 1);
				assert_eq!(c + 1, 1 << 18);
				match Self::get_pte_alloc_giga(address) {
					Ok(mut pte) => unsafe {
						if let Err(e) = pte.as_mut().set(map, rwx, usermode, global) {
							return undo(e);
						}
						address = NonNull::new_unchecked(address.as_ptr().add(1 << 18));
					},
					Err(e) => return undo(e),
				}
			}
		} else if addr % (1 << 21) == 0
			&& PPNBox::from(ppn_min) % (1 << 9) == 0
			&& PPNBox::from(ppn_max) % (1 << 9) == 0
		{
			let mut undo = #[cold] |err: AddError| todo!();
			while let Some(map) = map_range.pop_base() {
				let c = map_range.forget_base((1 << 9) - 1);
				assert_eq!(c + 1, 1 << 9);
				match Self::get_pte_alloc_mega(address) {
					Ok(mut pte) => unsafe {
						if let Err(e) = pte.as_mut().set(map, rwx, usermode, global) {
							return undo(e);
						}
						address = NonNull::new_unchecked(address.as_ptr().add(1 << 9));
					},
					Err(e) => return undo(e),
				}
			}
		} else {
			let mut undo = #[cold] |err: AddError| todo!();
			while let Some(map) = map_range.pop_base() {
				match Self::get_pte_alloc(address) {
					Ok(mut pte) => unsafe {
						if let Err(e) = pte.as_mut().set(map, rwx, usermode, global) {
							return undo(e);
						}
						address = NonNull::new_unchecked(address.as_ptr().add(1));
					},
					Err(e) => return undo(e),
				}
			}
		}
		Ok(())
	}

	/// Remove a mapping and return the original PPN.
	/// 
	/// ## Returns
	///
	/// * `Ok(PPN)` if the mapping existed and was removed successfully.
	/// * `Err(())` if the mapping doesn't exist.
	#[allow(dead_code)]
	pub fn remove(address: NonNull<Page>) -> Result<PrivateOrShared, ()> {
		unsafe { Self::get_pte(address).map_err(|_| ())?.as_mut().clear() }
	}

	/// Alias an address to another address.
	///
	/// The flags can optionally be changed (left to right: RWX, usermode)
	/// 
	/// ## Returns
	///
	/// * `Ok(())` if the address has been moved successfully.
	/// * `Err(())` if the source address doesn't map to a page.
	/// * `Err(())` if the destination address is already occupied.
	// FIXME is copy, should be move
	pub fn alias_address(from: NonNull<Page>, to: NonNull<Page>, rwx: RWX, usermode: bool, global: bool) -> Result<(), AddError> {

		if from == to {
			todo!()
		}

		let from = unsafe { Self::get_pte(from)?.as_mut() };
		let to = unsafe { Self::get_pte_alloc(to)?.as_mut() };

		if from.is_valid() {
			if !to.is_valid() {
				to.set(from.share().map_err(|_| todo!())?, rwx, usermode, global)
			} else {
				todo!()
			}
		} else {
			todo!()
		}
	}

	/// Write the physical *addresses* from the start of the virtual address into the given slice.
	pub fn physical_addresses(address: NonNull<Page>, store: &mut [usize]) -> Result<(), ()> {
		for (i, s) in store.iter_mut().enumerate() {
			let va = VirtualAddress(address.as_ptr().wrapping_add(i) as u64);

			// VPN[2]
			let pte = &unsafe { ROOT.as_ref() }[va.ppn_2()];
			if !pte.is_valid() {
				return Err(());
			} else if !pte.is_table() {
				*s = (((pte.0 & !0x3ff) << 2) | (va.0 & ((1 << 30) - 1))) as usize;
				continue;
			}

			// VPN[1]
			let ppn = unsafe { PPN::from_raw((pte.0 >> 10) as u32) };
			unsafe { Self::map_highmem_a(Some(&ppn)) };
			Self::flush_highmem_a();
			let tbl = unsafe { Self::translate_highmem_a(&ppn).cast::<[Entry; 512]>() };
			let tbl = &mut unsafe { &mut *tbl.as_ptr() };
			let pte = &mut tbl[va.ppn_1()];
			if !pte.is_valid() {
				return Err(());
			} else if !pte.is_table() {
				*s = (((pte.0 & !0x3ff) << 2) | (va.0 & ((1 << 21) - 1))) as usize;
				continue;
			}

			// VPN[0]
			let ppn = unsafe { PPN::from_raw((pte.0 >> 10) as u32) };
			unsafe { Self::map_highmem_a(Some(&ppn)) };
			Self::flush_highmem_a();
			let tbl = unsafe { Self::translate_highmem_a(&ppn).cast::<[Leaf; 512]>() };
			let tbl = &mut unsafe { &mut *tbl.as_ptr() };
			let pte = &mut tbl[va.ppn_0()];
			*s = ((pte.0 & !0x3ff) << 2) as usize;
		}
		Ok(())
	}

	/// Begin mapping a range of pages with PPNs passed from a function. Some of the PPNs may be
	/// used as tables.
	///
	/// This function never invokes the memory allocator directly and requires the passed PPNs to
	/// be identity mapped *and* not in any range of reserved memory.
	///
	/// It is intended only to be used by `crate::memory`. Use the other functions for regular
	/// allocations.
	pub fn allocate_pages<F>(mut f: F, address: NonNull<Page>, count: usize)
	where
		F: FnMut() -> PPN
	{
		// Map the root table
		unsafe {
			let va = VirtualAddress(ROOT.as_ptr() as u64);

			let root: usize;
			asm!("
				csrr	{0}, satp
				slli	{0}, {0}, 12
			", out(reg) root);

			let ppn_0 = f();
			let ppn_1 = f();
			let ppn_2 = PPN::from_ptr(root);

			let ppn_0_ptr = ppn_0.as_ptr();
			let ppn_1_ptr = ppn_1.as_ptr();
			let ppn_2_ptr = ppn_2.as_ptr();

			let mut leaf = Leaf(0);
			leaf.set(Map::Private(PPN::from_ptr(root)), RWX::RW, false, true).unwrap();
			ppn_0_ptr.cast::<Leaf>().add(va.ppn_0()).write(leaf);
			ppn_1_ptr.cast::<Entry>().add(va.ppn_1()).write(Entry::new_table(ppn_0));
			ppn_2_ptr.cast::<Entry>().add(va.ppn_2()).write(Entry::new_table(ppn_1));
		}

		// Begin allocating pages now.
		let mut va = VirtualAddress(address.as_ptr() as u64);

		for _ in 0..count {

			// PPN[2]
			let pte = &mut unsafe { &mut *ROOT.as_ptr() }[va.ppn_2()];
			if !pte.is_valid() {
				// Create a new PPN[1] table and map it.
				*pte = Entry::new_table(f());
			} else if !pte.is_table() {
				panic!("Page overlaps with an existing page");
			}

			// PPN[1]
			let ppn = unsafe { PPN::from_raw((pte.0 >> 10) as u32) };
			unsafe { Self::map_highmem_a(Some(&ppn)) };
			Self::flush_highmem_a();
			let tbl = unsafe { Self::translate_highmem_a(&ppn).cast::<[Entry; 512]>() };
			let tbl = &mut unsafe { &mut *tbl.as_ptr() };
			let pte = &mut tbl[va.ppn_1()];
			if !pte.is_valid() {
				let e = f();
				*pte = Entry::new_table(e);
			} else if !pte.is_table() {
				panic!("Page overlaps with an existing page");
			}
			mem::forget(ppn);

			// PPN[0]
			let ppn = unsafe { PPN::from_raw((pte.0 >> 10) as u32) };
			unsafe { Self::map_highmem_a(Some(&ppn)) };
			Self::flush_highmem_a();
			let tbl = unsafe { Self::translate_highmem_a(&ppn).cast::<[Leaf; 512]>() };
			let tbl = unsafe { &mut *tbl.as_ptr() };
			let pte = &mut tbl[va.ppn_0()];
			pte.set(Map::Private(f()), RWX::RW, false, true).expect("Page overlaps with an existing page");
			mem::forget(ppn);

			va.0 += u64::try_from(arch::PAGE_SIZE).unwrap();
		}
		unsafe { Self::map_highmem_a(None); }
		Self::flush_highmem_a();
	}

	/// Clear the identity maps.
	/// 
	/// This **must** only be called once at the end of early boot.
	pub fn clear_identity_maps() {
		let root = unsafe { &mut *ROOT.as_ptr() };
		for i in 0..256 {
			root[i] = Entry::new_invalid();
		}
	}

	pub fn current() -> Self {
		let root: u64;
		unsafe { asm!("csrr {0}, satp", out(reg) root) };
		Self(root)
	}

	/// Set HIGHMEM_A to map to the given PPN.
	///
	/// ## Safety
	///
	/// If HIGHMEM_A is mapped to another address the TLB *must* be flushed after this call.
	/// There may not be any lingering mappings either for security and performance.
	unsafe fn map_highmem_a(ppn: Option<&PPN>) {
		let va = VirtualAddress(HIGHMEM_A.as_ptr() as u64);
		let root = &mut *ROOT.as_ptr();
		root[va.ppn_2()] = if let Some(ppn) = ppn {
			let ppn = ppn.as_usize() & !((1 << 30) - 1);
			Entry::new_leaf2(ppn, RWX::RW, false, false)
		} else {
			Entry::new_invalid()
		};
	}

	/// Set HIGHMEM_B to map to the given PPN.
	///
	/// ## Safety
	///
	/// If HIGHMEM_B is mapped to another address the TLB *must* be flushed after this call.
	/// There may not be any lingering mappings either for security and performance.
	unsafe fn map_highmem_b(ppn: Option<&PPN>) {
		let va = VirtualAddress(HIGHMEM_B.as_ptr() as u64);
		let root = &mut *ROOT.as_ptr();
		root[va.ppn_2()] = if let Some(ppn) = ppn {
			let ppn = ppn.as_usize() & !((1 << 30) - 1);
			Entry::new_leaf2(ppn, RWX::RW, false, false)
		} else {
			Entry::new_invalid()
		};
	}

	/// Translate the PPN mapped to HIGHMEM_A to a virtual address.
	///
	/// ## Safety
	///
	/// `map_highmem_a` has been called before with the same PPN.
	unsafe fn translate_highmem_a(ppn: &PPN) -> NonNull<Page> {
		NonNull::new_unchecked(HIGHMEM_A.as_ptr().add((ppn.as_usize() & ((1 << 30) - 1)) >> 12))
	}

	/// Translate the PPN mapped to HIGHMEM_A to a virtual address.
	///
	/// ## Safety
	///
	/// `map_highmem_a` has been called before with the same PPN.
	unsafe fn translate_highmem_b(ppn: &PPN) -> NonNull<Page> {
		NonNull::new_unchecked(HIGHMEM_B.as_ptr().add((ppn.as_usize() & ((1 << 30) - 1)) >> 12))
	}

	/// Flush HIGHMEM_A from the TLB.
	fn flush_highmem_a() {
		Self::flush(Some(HIGHMEM_A));
	}

	/// Flush HIGHMEM_B from the TLB.
	fn flush_highmem_b() {
		Self::flush(Some(HIGHMEM_B));
	}

	/// Flush the given address from the TLB. If address is `None`, the entire TLB
	/// is flushed.
	fn flush(address: Option<NonNull<Page>>) {
		let address = address.map(|p| p.as_ptr() as *const _).unwrap_or(core::ptr::null());
		unsafe { asm!("sfence.vma {0}, zero", in(reg) address); }
	}
}

#[cfg(test)]
mod test {
	use super::*;

	test!(regular() {
		let mut sv = Sv39::new().unwrap();

		let pa_0 = NonNull::new(0x1000 as *mut _).unwrap();
		let va_0 = NonNull::new(0x2000 as *mut _).unwrap();

		let pa_0_5 = NonNull::new(0x1234 as *mut _).unwrap();
		let va_0_5 = NonNull::new(0x2234 as *mut _).unwrap();

		let pa_1 = NonNull::new(0x1000 as *mut _).unwrap();
		let va_1 = NonNull::new(0x1000 as *mut _).unwrap();

		let pa_2 = NonNull::new(0x111000 as *mut _).unwrap();
		let va_2 = NonNull::new(0x200000 as *mut _).unwrap();

		sv.add(
			Area::new(va_0.cast(), 0).unwrap(),
			Area::new(pa_0.cast(), 0).unwrap(),
			RWX::R,
			).unwrap();
		assert_eq!(sv.get(va_0), Some((pa_0, RWX::R)));
		assert_eq!(sv.get(va_0_5), Some((pa_0_5, RWX::R)));
		assert_eq!(sv.get(va_1), None);
		assert_eq!(sv.get(va_2), None);

		sv.add(
			Area::new(va_0.cast(), 0).unwrap(),
			Area::new(pa_1.cast(), 0).unwrap(),
			RWX::RX,
			).unwrap_err();
		assert_eq!(sv.get(va_0), Some((pa_0, RWX::R)));
		assert_eq!(sv.get(va_0_5), Some((pa_0_5, RWX::R)));
		assert_eq!(sv.get(va_1), None);
		assert_eq!(sv.get(va_2), None);

		sv.add(
			Area::new(va_1.cast(), 0).unwrap(),
			Area::new(pa_1.cast(), 0).unwrap(),
			RWX::RX,
			).unwrap();
		assert_eq!(sv.get(va_0), Some((pa_0, RWX::R)));
		assert_eq!(sv.get(va_0_5), Some((pa_0_5, RWX::R)));
		assert_eq!(sv.get(va_1), Some((pa_1, RWX::RX)));
		assert_eq!(sv.get(va_2), None);

		sv.add(
			Area::new(va_2.cast(), 0).unwrap(),
			Area::new(pa_2.cast(), 0).unwrap(),
			RWX::X,
			).unwrap();
		assert_eq!(sv.get(va_0), Some((pa_0, RWX::R)));
		assert_eq!(sv.get(va_0_5), Some((pa_0_5, RWX::R)));
		assert_eq!(sv.get(va_1), Some((pa_1, RWX::RX)));
		assert_eq!(sv.get(va_2), Some((pa_2, RWX::X)));
	});

	test!(regular_ord_1() {
		let mut sv = Sv39::new().unwrap();

		let pa_0 = NonNull::new(0x10000 as *mut _).unwrap();
		let pa_1 = NonNull::new(0x11000 as *mut _).unwrap();

		let va_0 = NonNull::new(0x2000 as *mut _).unwrap();
		let va_1 = NonNull::new(0x3000 as *mut _).unwrap();

		sv.add(
			Area::new(va_0.cast(), 1).unwrap(),
			Area::new(pa_0.cast(), 1).unwrap(),
			RWX::R,
			).unwrap();
		assert_eq!(sv.get(va_0), Some((pa_0, RWX::R)));
		assert_eq!(sv.get(va_1), Some((pa_1, RWX::R)));

		sv.add(
			Area::new(va_1.cast(), 0).unwrap(),
			Area::new(pa_1.cast(), 0).unwrap(),
			RWX::RX,
			).unwrap_err();
		assert_eq!(sv.get(va_0), Some((pa_0, RWX::R)));
		assert_eq!(sv.get(va_1), Some((pa_1, RWX::R)));
	});

	test!(mega() {
		let mut sv = Sv39::new().unwrap();

		let pa_0 = NonNull::new(0x20_0000 as *mut _).unwrap();
		let pa_0_5 = NonNull::new(0x30_1234 as *mut _).unwrap();
		let pa_1 = NonNull::new(0x100_0000 as *mut _).unwrap();
		let pa_2 = NonNull::new(0x180_0000 as *mut _).unwrap();

		let va_0 = NonNull::new(0x200_0000 as *mut _).unwrap();
		let va_0_5 = NonNull::new(0x210_1234 as *mut _).unwrap();
		let va_1 = NonNull::new(0x280_0000 as *mut _).unwrap();
		let va_2 = NonNull::new(0x1000_0000 as *mut _).unwrap();

		sv.add(
			Area::new(va_0.cast(), 9).unwrap(),
			Area::new(pa_0.cast(), 9).unwrap(),
			RWX::R,
			).unwrap();
		assert_eq!(sv.get(va_0), Some((pa_0, RWX::R)));
		assert_eq!(sv.get(va_0_5), Some((pa_0_5, RWX::R)));
		assert_eq!(sv.get(va_1), None);
		assert_eq!(sv.get(va_2), None);

		sv.add(
			Area::new(va_1.cast(), 9).unwrap(),
			Area::new(pa_1.cast(), 9).unwrap(),
			RWX::RX,
			).unwrap();
		assert_eq!(sv.get(va_0), Some((pa_0, RWX::R)));
		assert_eq!(sv.get(va_0_5), Some((pa_0_5, RWX::R)));
		assert_eq!(sv.get(va_1), Some((pa_1, RWX::RX)));
		assert_eq!(sv.get(va_2), None);

		sv.add(
			Area::new(va_2.cast(), 9).unwrap(),
			Area::new(pa_2.cast(), 9).unwrap(),
			RWX::RW,
			).unwrap();
		assert_eq!(sv.get(va_0), Some((pa_0, RWX::R)));
		assert_eq!(sv.get(va_0_5), Some((pa_0_5, RWX::R)));
		assert_eq!(sv.get(va_1), Some((pa_1, RWX::RX)));
		assert_eq!(sv.get(va_2), Some((pa_2, RWX::RW)));
	});

	test!(giga() {
		let mut sv = Sv39::new().unwrap();

		let pa_0 = NonNull::new(0x4000_0000 as *mut _).unwrap();
		let pa_0_5 = NonNull::new(0x4fed_1234 as *mut _).unwrap();
		let pa_1 = NonNull::new(0x8000_0000 as *mut _).unwrap();
		let pa_2 = NonNull::new(0x1_8000_0000 as *mut _).unwrap();

		let va_0 = NonNull::new(0x14000_0000 as *mut _).unwrap();
		let va_0_5 = NonNull::new(0x14fed_1234 as *mut _).unwrap();
		let va_1 = NonNull::new(0x2_0000_0000 as *mut _).unwrap();
		let va_2 = NonNull::new(0x1_8000_0000 as *mut _).unwrap();

		sv.add(
			Area::new(va_0.cast(), 18).unwrap(),
			Area::new(pa_0.cast(), 18).unwrap(),
			RWX::R,
		).unwrap();
		assert_eq!(sv.get(va_0), Some((pa_0, RWX::R)));
		assert_eq!(sv.get(va_0_5), Some((pa_0_5, RWX::R)));
		assert_eq!(sv.get(va_1), None);
		assert_eq!(sv.get(va_2), None);

		sv.add(
			Area::new(va_1.cast(), 18).unwrap(),
			Area::new(pa_1.cast(), 18).unwrap(),
			RWX::RX,
		).unwrap();
		assert_eq!(sv.get(va_0), Some((pa_0, RWX::R)));
		assert_eq!(sv.get(va_0_5), Some((pa_0_5, RWX::R)));
		assert_eq!(sv.get(va_1), Some((pa_1, RWX::RX)));
		assert_eq!(sv.get(va_2), None);

		sv.add(
			Area::new(va_2.cast(), 18).unwrap(),
			Area::new(pa_2.cast(), 18).unwrap(),
			RWX::RW,
		).unwrap();
		assert_eq!(sv.get(va_0), Some((pa_0, RWX::R)));
		assert_eq!(sv.get(va_0_5), Some((pa_0_5, RWX::R)));
		assert_eq!(sv.get(va_1), Some((pa_1, RWX::RX)));
		assert_eq!(sv.get(va_2), Some((pa_2, RWX::RW)));
	});

	test!(shared() {
		let mut sv = Sv39::new().unwrap();

		let va_0 = NonNull::new(0x14000_0000 as *mut _).unwrap();

		sv.allocate_shared(va_0, 3, RWX::R);
		assert_eq!(sv.get(va_0.cast()).unwrap().1, RWX::R);
		assert!(sv.is_shared(va_0.cast()));
	});

	test!(mixed() {
		let mut sv = Sv39::new().unwrap();

		let pa_0 = NonNull::new(0x4000_0000 as *mut _).unwrap();
		let pa_0_5 = NonNull::new(0x4fed_1234 as *mut _).unwrap();
		let pa_1 = NonNull::new(0x8000_0000 as *mut _).unwrap();
		let pa_2 = NonNull::new(0x1_8000_0000 as *mut _).unwrap();

		let va_0 = NonNull::new(0x14000_0000 as *mut _).unwrap();
		let va_0_5 = NonNull::new(0x14fed_1234 as *mut _).unwrap();
		let va_1 = NonNull::new(0x2_0000_0000 as *mut _).unwrap();
		let va_2 = NonNull::new(0x1_8000_0000 as *mut _).unwrap();

		sv.add(
			Area::new(va_0.cast(), 18).unwrap(),
			Area::new(pa_0.cast(), 18).unwrap(),
			RWX::R,
		).unwrap();
		assert_eq!(sv.get(va_0), Some((pa_0, RWX::R)));
		assert_eq!(sv.get(va_0_5), Some((pa_0_5, RWX::R)));
		assert_eq!(sv.get(va_1), None);
		assert_eq!(sv.get(va_2), None);

		sv.add(
			Area::new(va_1.cast(), 10).unwrap(),
			Area::new(pa_1.cast(), 10).unwrap(),
			RWX::RX,
		).unwrap();
		assert_eq!(sv.get(va_0), Some((pa_0, RWX::R)));
		assert_eq!(sv.get(va_0_5), Some((pa_0_5, RWX::R)));
		assert_eq!(sv.get(va_1), Some((pa_1, RWX::RX)));
		assert_eq!(sv.get(va_2), None);

		sv.add(
			Area::new(va_2.cast(), 2).unwrap(),
			Area::new(pa_2.cast(), 2).unwrap(),
			RWX::RW,
		).unwrap();
		assert_eq!(sv.get(va_0), Some((pa_0, RWX::R)));
		assert_eq!(sv.get(va_0_5), Some((pa_0_5, RWX::R)));
		assert_eq!(sv.get(va_1), Some((pa_1, RWX::RX)));
		assert_eq!(sv.get(va_2), Some((pa_2, RWX::RW)));
	});
}
