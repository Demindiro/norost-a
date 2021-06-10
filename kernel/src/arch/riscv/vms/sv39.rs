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
use crate::arch::{Page, VirtualMemorySystem};
use crate::memory::{self, AllocateError, PPN};
use crate::memory::reserved::{GLOBAL, LOCAL, VMM_PPN0, VMM_PPN1, VMM_PPN2};
use core::convert::TryFrom;
use core::mem;
use core::ptr::NonNull;
use core::ops;

/// The start index of the global kernel table.
const GLOBAL_KERNEL_TABLE_START_INDEX: usize = unsafe {
	(GLOBAL.start.as_ptr() as usize >> 30) & 0x1ff
};

/// The start index of the local kernel table.
const LOCAL_KERNEL_TABLE_START_INDEX: usize = unsafe {
	(LOCAL.start.as_ptr() as usize >> 30) & 0x1ff
};

/// Page table entry
///
/// The format from MSb to LSb is:
///
/// - 10 bits reserved
/// - 26 bits PPN[2]
/// - 9 bits PPN[1]
/// - 9 bits PPN[0]
/// - 2 bits RSW, free for us to use
///   - The MSb indicates whether the page's RWX flags are locked.
///   - The LSb is currently unused.
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

enum Either {
	Table(TablePage),
	Address((PhysicalAddress, RWX)),
}

/// Page table.
///
/// Each table contains 512 entries.
#[repr(align(4096))]
struct Table([Entry; 512]);

/// Page-allocated page table.
struct TablePage(NonNull<Page>);

struct VirtualAddress(u64);

struct PhysicalAddress(u64);

/// The root table of a Sv39 VMS.
#[repr(C)]
pub struct Sv39 {
	/// The address mappings.
	addresses: TablePage,
}

impl Entry {
	const PAGE_MASK: u64 = arch::PAGE_MASK as u64;

	const VALID_MASK: u64 = 0b1;

	const RWX_MASK: u64 = 0b1110;
	const _RWX_MASK_CHECK: u64 = 0 - (Self::RWX_MASK - RWX::MASK_64);

	const USERMODE_MASK: u64 = 0b1_0000;

	const SHARED_MASK: u64 = 0x1_00;

	const PPN_2_OFFSET: u8 = 28;
	const PPN_1_OFFSET: u8 = 19;
	const PPN_0_OFFSET: u8 = 10;

	const PPN_2_MASK: u64 = 0x3ff_ffff << Self::PPN_2_OFFSET;
	const PPN_1_MASK: u64 = 0x1ff << Self::PPN_1_OFFSET;
	const PPN_0_MASK: u64 = 0x1ff << Self::PPN_0_OFFSET;

	/// Create a new entry for an invalid entry.
	fn new_invalid() -> Self {
		Self(0)
	}

	/// Create a new entry for a single physical entry.
	#[must_use]
	fn new_leaf(ppn: PPN, rwx: RWX) -> Self {
		let ppn = ppn.into_raw();
		let mut s = Self((ppn as u64) << 10);
		s.set_rwx(rwx);
		s.set_valid(true);
		s.set_usermode(true);
		s.0 |= 0b1100_0000;
		s
	}

	/// Create a new entry for a single table entry.
	#[must_use]
	fn new_table(table: TablePage) -> Self {
		let s = table.0.as_ptr() as u64;
		let mut s = Self(s >> 2);
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

	/// Set whether this page is shared.
	fn set_shared(&mut self, shared: bool) {
		self.0 &= !Self::SHARED_MASK;
		self.0 |= u64::from(shared) * Self::SHARED_MASK;
	}

	/// Return as a pointer to a page table.
	#[must_use]
	fn as_table(&self) -> Option<TablePage> {
		if self.is_valid() && self.rwx().is_none() {
			debug_assert_ne!(self.0 & !Self::PAGE_MASK, 0, "Table pointer is null");
			let s = self.0 >> Self::PPN_0_OFFSET;
			let s = s << 12;
			unsafe {
				Some(TablePage(NonNull::new_unchecked(s as *mut _)))
			}
		} else {
			None
		}
	}

	/// Return as a physical address.
	#[must_use]
	fn as_address(&self) -> Option<(PhysicalAddress, RWX)> {
		if self.is_valid() {
			self.rwx().map(|rwx| (
					PhysicalAddress::new(self.ppn_2(), self.ppn_1(), self.ppn_0(), 0),
					rwx
					))
		} else {
			None
		}
	}

	/// Return as either a physical address or a page table
	#[must_use]
	fn as_either(&self) -> Option<Either> {
		if let Some(tbl) = self.as_table() {
			Some(Either::Table(tbl))
		} else {
			self.as_address().map(Either::Address)
		}
	}

	/// Return the PPN[2] shifted to the right.
	#[must_use]
	fn ppn_2(&self) -> u64 {
		(self.0 & Self::PPN_2_MASK) >> Self::PPN_2_OFFSET
	}

	/// Return the PPN[1] shifted to the right.
	#[must_use]
	fn ppn_1(&self) -> u64 {
		(self.0 & Self::PPN_1_MASK) >> Self::PPN_1_OFFSET
	}

	/// Return the PPN[0] shifted to the right.
	#[must_use]
	fn ppn_0(&self) -> u64 {
		(self.0 & Self::PPN_0_MASK) >> Self::PPN_0_OFFSET
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

impl TablePage {
	/// Create a new table with empty (zeroed) entries.
	fn new() -> Result<Self, AllocateError> {
		// FIXME
		let a = memory::mem_allocate(0)?;
		let a = unsafe { core::mem::transmute::<_, u32>(a) };
		let a = a as usize;
		let a = a << 12;
		let a = a as *mut _;
		let a = NonNull::new(a).unwrap();
		Ok(Self(a))
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
	const OFFSET_MASK: u64 = 0x3ff;

	const PPN_2_OFFSET: u64 = 30;
	const PPN_1_OFFSET: u64 = 21;
	const PPN_0_OFFSET: u64 = 12;

	const PPN_2_MASK: u64 = 0x1ff << Self::PPN_2_OFFSET;
	const PPN_1_MASK: u64 = 0x1ff << Self::PPN_1_OFFSET;
	const PPN_0_MASK: u64 = 0x1ff << Self::PPN_0_OFFSET;

	/// Return the PPN[2] shifted to the right.
	fn ppn_2(&self) -> u64 {
		(self.0 & Self::PPN_2_MASK) >> Self::PPN_2_OFFSET
	}

	/// Return the PPN[1] shifted to the right.
	fn ppn_1(&self) -> u64 {
		(self.0 & Self::PPN_1_MASK) >> Self::PPN_1_OFFSET
	}

	/// Return the PPN[0] shifted to the right.
	fn ppn_0(&self) -> u64 {
		(self.0 & Self::PPN_0_MASK) >> Self::PPN_0_OFFSET
	}

	/// Return the PPN[2:1] shifted to the right.
	fn ppn_21(&self) -> u64 {
		(self.0 & (Self::PPN_2_MASK | Self::PPN_1_MASK)) >> Self::PPN_1_OFFSET
	}

	/// Return the PPN[2:0] shifted to the right.
	fn ppn_210(&self) -> u64 {
		(self.0 & (Self::PPN_2_MASK | Self::PPN_1_MASK | Self::PPN_0_MASK)) >> Self::PPN_0_OFFSET
	}

	/// Return the offset
	fn offset(&self) -> u64 {
		self.0 & Self::OFFSET_MASK
	}
}

impl PhysicalAddress {
	const OFFSET_MASK: u64 = 0x3ff;

	const PPN_2_OFFSET: u64 = 30;
	const PPN_1_OFFSET: u64 = 21;
	const PPN_0_OFFSET: u64 = 12;

	const PPN_2_MASK: u64 = 0x3ff_ffff << Self::PPN_2_OFFSET;
	const PPN_1_MASK: u64 = 0x1ff << Self::PPN_1_OFFSET;
	const PPN_0_MASK: u64 = 0x1ff << Self::PPN_0_OFFSET;

	/// Creates a new physical address from the PPNs and the offset.
	fn new(ppn_2: u64, ppn_1: u64, ppn_0: u64, offset: u64) -> Self {
		let r = (ppn_2 << Self::PPN_2_OFFSET) | (ppn_1 << Self::PPN_1_OFFSET) | (ppn_0 << Self::PPN_0_OFFSET) | offset;
		Self(r)
	}

	/// Return the PPN[2] shifted to the right.
	fn ppn_2(&self) -> u64 {
		(self.0 & Self::PPN_2_MASK) >> Self::PPN_2_OFFSET
	}

	/// Return the PPN[1] shifted to the right.
	fn ppn_1(&self) -> u64 {
		(self.0 & Self::PPN_1_MASK) >> Self::PPN_1_OFFSET
	}

	/// Return the PPN[0] shifted to the right.
	fn ppn_0(&self) -> u64 {
		(self.0 & Self::PPN_0_MASK) >> Self::PPN_0_OFFSET
	}

	/// Return the offset
	fn offset(&self) -> u64 {
		self.0 & Self::OFFSET_MASK
	}
}

impl Sv39 {
	const MEGA_PAGE_ORDER: u8 = 9;
	const GIGA_PAGE_ORDER: u8 = 18;

	const PAGE_SIZE: u64 = 4096;
	const MEGA_PAGE_SIZE: u64 = Self::PAGE_SIZE << Self::MEGA_PAGE_ORDER;
	const GIGA_PAGE_SIZE: u64 = Self::PAGE_SIZE << Self::GIGA_PAGE_ORDER;

	const PAGE_MASK: u64 = Self::PAGE_SIZE - 1;
	const MEGA_PAGE_MASK: u64 = Self::MEGA_PAGE_SIZE - 1;
	const GIGA_PAGE_MASK: u64 = Self::GIGA_PAGE_SIZE - 1;

	const ENTRIES_PER_TABLE: u64 = 512;

	/// Create a new Sv39 mapping.
	pub fn new() -> Result<Self, AllocateError> {
		let mut tp = TablePage::new()?;
		unsafe {
			let mut global: u64;
			asm!("
				csrr	t0, satp
				slli	t0, t0, 12
			", out("t0") global);
			let global: TablePage = core::mem::transmute(global);
			for i in 1..512 {
				tp[i] = core::mem::transmute::<_, _>(*core::mem::transmute::<_, &u64>(&global[i]));
			}
		}
		Ok(Self {
			addresses: tp,
		})
	}

	/// Allocate the given amount of private pages and insert it as virtual memory at the
	/// given address.
	pub fn allocate(&mut self, virtual_address: NonNull<Page>, count: usize, rwx: RWX) -> Result<(), AddError> {
		let mut va = virtual_address;
		// FIXME deallocate pages on failure.
		memory::mem_allocate_range(count, |ppn| {
			Self::add(va, ppn, rwx, true);
			va = NonNull::new(va.as_ptr().wrapping_add(1)).unwrap();
		}).unwrap();
		Ok(())
	}

	/// Add a mapping. If no virtual address is given, the first available
	/// entry with enough space is used.
	pub fn add(virtual_address: NonNull<Page>, ppn: PPN, rwx: RWX, user: bool) -> Result<(), AddError> {

		let mut s: Self = unsafe {
			let s: usize;
			asm!("
				csrr	t0, satp
				slli	t0, t0, 12
			", out("t0") s);
			core::mem::transmute(s)
		};

		let va = VirtualAddress(virtual_address.as_ptr() as u64);
		let index = va.ppn_210();

		let e = &mut s.addresses[index >> 18];
		let mut tbl = if let Some(tbl) = e.as_table() {
			tbl
		} else if !e.is_valid() {
			*e = Entry::new_table(TablePage::new().expect("TODO"));
			e.as_table().unwrap()
		} else {
			// FIXME we should be unmapping the identity maps elsewhere...
			//return Err(AddError::Overlaps);
			*e = Entry::new_table(TablePage::new().expect("TODO"));
			unsafe { asm!("sfence.vma"); }
			e.as_table().unwrap()
		};

		let e = &mut tbl[(index >> 9) & 0x1ff];
		let mut tbl = if let Some(tbl) = e.as_table() {
			tbl
		} else if !e.is_valid() {
			*e = Entry::new_table(TablePage::new().expect("TODO"));
			e.as_table().unwrap()
		} else {
			return Err(AddError::Overlaps);
		};

		let e = &mut tbl[index & 0x1ff];
		if e.is_valid() {
			return Err(AddError::Overlaps);
		}

		*e = Entry::new_leaf(ppn, rwx);
		e.set_usermode(user);

		Ok(())
	}

	/// Allocate and add a shared mapping.
	pub fn allocate_shared(&mut self, address: NonNull<Page>, count: usize, rwx: RWX) -> Result<(), ()> {
		let mut va = address;
		// FIXME deallocate pages on failure.
		memory::mem_allocate_range(count, |ppn| {
			let ppn = crate::memory::SharedPage::new(ppn).unwrap();
			let ppn = ppn.into_raw();
			Self::add(va, ppn, rwx, true).unwrap();
			let index = (va.as_ptr() as usize >> 12) as u64;
			va = NonNull::new(va.as_ptr().wrapping_add(1)).unwrap();
		}).unwrap();
		Ok(())
	}

	/// Remove a mapping and deallocate the associated memory.
	/// 
	/// ## Returns
	///
	/// `Ok(())` if the mapping existed and was removed successfully.
	/// `Err(Invalid)` if the mapping doesn't exist.
	pub fn deallocate(&mut self, virtual_address: NonNull<Page>, count: usize) -> Result<(), ()> {
		todo!()
	}

	/// Change the address of a physical page.
	///
	/// The flags can optionally be changed (left to right: RWX, usermode)
	/// 
	/// ## Returns
	///
	/// * `Ok(())` if the address has been moved successfully.
	/// * `Err(())` if the source address doesn't map to a page.
	/// * `Err(())` if the destination address is already occupied.
	// FIXME is copy, should be move
	pub fn copy_address(from: NonNull<Page>, to: NonNull<Page>, flags: Option<(RWX, bool)>) -> Result<(), ()> {
		let mut addresses: TablePage = unsafe {
			let s: usize;
			asm!("
				csrr	t0, satp
				slli	t0, t0, 12
			", out("t0") s);
			core::mem::transmute(s)
		};

		let from = VirtualAddress(from.as_ptr() as u64);
		let to = VirtualAddress(to.as_ptr() as u64);

		// Get the source entry and zero it out.
		let mut tbl = &mut addresses;
		let mut tbl = tbl[from.ppn_2()].as_table().ok_or(())?;
		let mut tbl = tbl[from.ppn_1()].as_table().ok_or(())?;
		let mut entry = Entry::new_invalid();
		// FIXME
		//mem::swap(&mut tbl[from.ppn_0()], &mut entry);
		entry = unsafe { mem::transmute_copy(&tbl[from.ppn_0()]) };
		if !entry.is_valid() {
			return Err(());
		}
		let mut tbl2 = tbl;

		// Move the source entry.
		let mut tbl = &mut addresses;

		// FIXME
		if tbl[to.ppn_2()].is_valid() && tbl[to.ppn_2()].as_table().is_none() {
			tbl[to.ppn_2()] = Entry::new_table(TablePage::new().unwrap());
		}

		if let Some(mut tbl) = tbl[to.ppn_2()].as_table() {
			if !tbl[to.ppn_1()].is_valid() {
				tbl[to.ppn_1()] = Entry::new_table(TablePage::new().unwrap());
			}
			if let Some(mut tbl) = tbl[to.ppn_1()].as_table() {
				let e = &mut tbl[to.ppn_0()];
				if !e.is_valid() {
					*e = entry;
					if let Some((rwx, u)) = flags {
						e.set_rwx(rwx);
						e.set_usermode(u);
					}
					Ok(())
				} else {
					mem::swap(&mut tbl2[from.ppn_0()], &mut entry);
					Err(())
				}
			} else {
				mem::swap(&mut tbl2[from.ppn_0()], &mut entry);
				Err(())
			}
		} else {
			mem::swap(&mut tbl2[from.ppn_0()], &mut entry);
			Err(())
		}
	}

	/// Add a kernel mapping
	///
	/// Kernel mappings are global, hence this parameter doesn't take `self` but instead reads
	/// `satp`
	pub fn add_kernel_mapping<F: FnMut() -> crate::memory::PPN>(mut f: F, count: usize, rwx: RWX) -> NonNull<Page> {
		// FIXME HACKS HACKS HACKS AAAAAAA
		let virtual_address = NonNull::<Page>::new(0xffff_ffff_ffe0_0000 as *mut _).unwrap();
		let va = VirtualAddress(virtual_address.as_ptr() as u64);
		// PPN[2]
		let tbl: TablePage = unsafe {
			let s: usize;
			asm!("
				csrr	t0, satp
				slli	t0, t0, 12
			", out("t0") s);
			core::mem::transmute(s)
		};
		// PPN[1]
		let tbl = tbl[511].as_table().unwrap();
		// PPN[0]
		let mut tbl = tbl[511].as_table().unwrap();
		for i in 0..count as u64 {
			assert!(tbl[i].as_either().is_none(), "FUCK ME BBLECIOHRGOIHRG");
			let mut e = Entry::new_leaf(f(), rwx);
			e.set_usermode(false);
			tbl[i] = e;
		}
		virtual_address
	}

	pub fn current() -> Self {
		unsafe {
			let s: usize;
			asm!("
				csrr	{0}, satp
				slli	{0}, {0}, 12
			", out(reg) s);
			core::mem::transmute(s)
		}
	}
}

impl Drop for Sv39 {
	fn drop(&mut self) {
		/*
		// PPN[2]
		for e in self.addresses.iter() {
			if let Some(tbl) = e.as_table() {
				// PPN[1]
				for e in tbl.iter() {
					if let Some(tbl) = e.as_table() {
						// PPN[0]
						let a = tbl.0
						// SAFETY: We own a unique reference to a valid page.
						unsafe {
							memory::mem_deallocate(a);
						}
					}
				}
				let a = Area::new(tbl.0, 0).unwrap();
				// SAFETY: We own a unique reference to a valid page.
				unsafe {
					memory::mem_deallocate(a);
				}
			}
		}
		let a = Area::new(self.addresses.0, 0).unwrap();
		// SAFETY: We own a unique reference to a valid page.
		unsafe {
			memory::mem_deallocate(a);
		}
		*/

		// FIXME
		//todo!()
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
