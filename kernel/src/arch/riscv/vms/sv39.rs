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
use crate::MEMORY_MANAGER;
use crate::memory::{Area, AllocateError};
use core::convert::TryFrom;
use core::ptr::NonNull;
use core::ops;

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
pub struct Sv39(TablePage);

impl Entry {
	const PAGE_MASK: u64 = arch::PAGE_MASK as u64;

	const VALID_MASK: u64 = 0b1;

	const RWX_MASK: u64 = 0b1110;
	const _RWX_MASK_CHECK: u64 = 0 - (Self::RWX_MASK - RWX::MASK_64);

	const USERMODE_MASK: u64 = 0b1_0000;

	const PPN_2_OFFSET: u8 = 28;
	const PPN_1_OFFSET: u8 = 19;
	const PPN_0_OFFSET: u8 = 10;

	const PPN_2_MASK: u64 = 0x3ff_ffff << Self::PPN_2_OFFSET;
	const PPN_1_MASK: u64 = 0x1ff << Self::PPN_1_OFFSET;
	const PPN_0_MASK: u64 = 0x1ff << Self::PPN_0_OFFSET;

	const INVALID: Self = Self(0);

	/// Create a new entry for a single physical entry.
	#[must_use]
	fn new_leaf(physical_address: PhysicalAddress, rwx: RWX) -> Self {
		let mut s = Self(0);
		s.0 |= physical_address.ppn_2() << Self::PPN_2_OFFSET;
		s.0 |= physical_address.ppn_1() << Self::PPN_1_OFFSET;
		s.0 |= physical_address.ppn_0() << Self::PPN_0_OFFSET;
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
		s.set_usermode(true);
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
		Ok(Self(MEMORY_MANAGER.lock().allocate(0)?.start()))
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
		Ok(Self(TablePage::new()?))
	}

	/// Add a mapping. If no virtual address is given, the first available
	/// entry with enough space is used.
	///
	/// ## Returns
	///
	/// `Ok(NonNull<Page>)` if the mapping was not yet in use.
	/// `Err(AlreadyUsed)` if the mapping was already in use.
	/// `Err(NoSpace)` if there is no address range that is large enough.
	/// `Err(OutOfBounds)` if the `order` is too large for the given address.
	pub fn add(&mut self, virtual_address: Area, physical_address: Area, rwx: RWX) -> Result<(), AddError> {
		if virtual_address.order() != physical_address.order() {
			return Err(AddError::NonEqualOrder);
		}

		let order = virtual_address.order();
		let va = virtual_address.start();
		let pa = physical_address.start();

		if order >= Self::GIGA_PAGE_ORDER {
			let count = 1 << (order - Self::GIGA_PAGE_ORDER);
			let mut index = VirtualAddress(va.as_ptr() as u64).ppn_2();
			let end = (index + count) & (Self::ENTRIES_PER_TABLE - 1);
			if end <= index {
				todo!();
			}
			let mut va_curr = va.as_ptr() as u64;
			let mut pa_curr = pa.as_ptr() as u64;

			// Write entries as we go. If the range turns out to be too small, clear them.
			while index != end {
				let va = VirtualAddress(va_curr);
				let pa = PhysicalAddress(pa_curr);
				let e = &mut self.0[index];
				if e.is_valid() {
					// Clear previously written entries
					todo!()
				}
				*e = Entry::new_leaf(pa, rwx);
				va_curr += Self::GIGA_PAGE_SIZE;
				pa_curr += Self::GIGA_PAGE_SIZE;
				index += 1;
			}

		} else if order >= Self::MEGA_PAGE_ORDER {
			let count = 1 << (order - Self::MEGA_PAGE_ORDER);
			let start = VirtualAddress(va.as_ptr() as u64).ppn_21();
			let mut index = start;
			let end = (index + count) & (Self::ENTRIES_PER_TABLE * Self::ENTRIES_PER_TABLE - 1);
			if end <= index {
				todo!();
			}
			let mut va_curr = va.as_ptr() as u64;
			let mut pa_curr = pa.as_ptr() as u64;

			let clear = #[cold] #[inline(never)] |s: &mut Self, index, err| {
				for i in start..index {
					s.0[i >> 9]
						.as_table().unwrap()[i & 0x1ff] = Entry::INVALID;
				}
				Err(err)
			};

			// Write entries as we go. If the range turns out to be too small, clear them.
			while index != end {
				let e = &mut self.0[index >> 9];
				let mut tbl = if let Some(tbl) = e.as_table() {
					tbl
				} else if !e.is_valid() {
					*e = Entry::new_table(TablePage::new().expect("TODO"));
					e.as_table().unwrap()
				} else {
					// Clear previously written entries
					return clear(self, index, AddError::Overlaps);
				};

				let e = &mut tbl[index & (Self::ENTRIES_PER_TABLE - 1)];
				let va = VirtualAddress(va_curr);
				let pa = PhysicalAddress(pa_curr);
				*e = Entry::new_leaf(pa, rwx);
				va_curr += Self::MEGA_PAGE_SIZE;
				pa_curr += Self::MEGA_PAGE_SIZE;
				index += 1;
			}

		} else {

			let count = 1 << order;
			let start = VirtualAddress(va.as_ptr() as u64).ppn_210();
			let mut index = start;
			let end = (index + count) & (Self::ENTRIES_PER_TABLE * Self::ENTRIES_PER_TABLE * Self::ENTRIES_PER_TABLE - 1);
			if end <= index {
				todo!();
			}
			let mut va_curr = va.as_ptr() as u64;
			let mut pa_curr = pa.as_ptr() as u64;

			let clear = #[cold] #[inline(never)] |s: &mut Self, index, err| {
				for i in start..index {
					s.0[i >> 18]
						.as_table().unwrap()[(i >> 9) & 0x1ff]
						.as_table().unwrap()[i & 0x1ff] = Entry::INVALID;
				}
				Err(err)
			};

			// Write entries as we go. If the range turns out to be too small, clear them.
			while index != end {

				let e = &mut self.0[index >> 18];
				let mut tbl = if let Some(tbl) = e.as_table() {
					tbl
				} else if !e.is_valid() {
					*e = Entry::new_table(TablePage::new().expect("TODO"));
					e.as_table().unwrap()
				} else {
					// Clear previously written entries
					return clear(self, index, AddError::Overlaps);
				};

				let e = &mut tbl[(index >> 9) & 0x1ff];
				let mut tbl = if let Some(tbl) = e.as_table() {
					tbl
				} else if !e.is_valid() {
					*e = Entry::new_table(TablePage::new().expect("TODO"));
					e.as_table().unwrap()
				} else {
					// Clear previously written entries
					return clear(self, index, AddError::Overlaps);
				};

				let e = &mut tbl[index & 0x1ff];
				if e.is_valid() {
					// Clear previously written entries
					return clear(self, index, AddError::Overlaps);
				}

				let va = VirtualAddress(va_curr);
				let pa = PhysicalAddress(pa_curr);
				*e = Entry::new_leaf(pa, rwx);
				va_curr += Self::PAGE_SIZE;
				pa_curr += Self::PAGE_SIZE;
				index += 1;
			}
		}

		Ok(())
	}

	/// Remove a mapping.
	/// 
	/// ## Returns
	///
	/// `Ok(())` if the mapping existed and was removed successfully.
	/// `Err(Invalid)` if the mapping doesn't exist.
	pub fn remove(&mut self, virtual_address: NonNull<Page>, order: u8) -> Result<(), ()> {
		todo!()
	}

	/// Map a virtual address to a physical address.
	/// 
	/// ## Returns
	///
	/// `Ǹone` if the virtual address is not mapped.
	/// `Some((ppn_u, address))` if the address is mapped. `ppn_u` represents the upper
	/// 17 bits of the physical address.
	pub fn get(&self, virtual_address: NonNull<u8>) -> Option<(NonNull<u8>, RWX)> {
		let va = VirtualAddress(virtual_address.as_ptr() as u64);
		
		let entry = &self.0[va.ppn_2()];
		let (ppn_2, ppn_1, ppn_0, rwx) = match entry.as_either()? {
			Either::Table(tbl) => {
				let entry = &tbl[va.ppn_1()];
				match entry.as_either()? {
					Either::Table(tbl) => {
						let entry = &tbl[va.ppn_0()];
						entry
							.as_address()
							.map(|(pa, rwx)| (pa.ppn_2(), pa.ppn_1(), pa.ppn_0(), rwx))?
					}
					Either::Address((pa, rwx)) => {
						(pa.ppn_2(), pa.ppn_1(), va.ppn_0(), rwx)
					}
				}
			}
			Either::Address((pa, rwx)) => (pa.ppn_2(), va.ppn_1(), va.ppn_0(), rwx),
		};

		let pa = PhysicalAddress::new(ppn_2, ppn_1, ppn_0, va.offset());
		debug_assert_ne!(pa.0, 0);
		Some((unsafe { NonNull::new_unchecked(pa.0 as *mut _) }, rwx))
	}
}

impl Drop for Sv39 {
	fn drop(&mut self) {
		// PPN[2]
		for e in self.0.iter() {
			if let Some(tbl) = e.as_table() {
				// PPN[1]
				for e in tbl.iter() {
					if let Some(tbl) = e.as_table() {
						// PPN[0]
						let a = Area::new(tbl.0, 0).unwrap();
						// SAFETY: We own a unique reference to a valid page.
						unsafe {
							MEMORY_MANAGER.lock().deallocate(a);
						}
					}
				}
				let a = Area::new(tbl.0, 0).unwrap();
				// SAFETY: We own a unique reference to a valid page.
				unsafe {
					MEMORY_MANAGER.lock().deallocate(a);
				}
			}
		}
		let a = Area::new(self.0.0, 0).unwrap();
		// SAFETY: We own a unique reference to a valid page.
		unsafe {
			MEMORY_MANAGER.lock().deallocate(a);
		}
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
