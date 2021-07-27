//! # Virtual Memory System
//!
//! A VMS is an essential part of any OS that uses MMUs. This module defines a trait that defines
//! all the methods that must be present for a VMS to be useable.

use super::*;
use crate::memory::{AllocateError, SharedPPN, PPN};

/// The accessibility of the mapping to be added.
#[derive(Clone, Copy)]
pub enum Accessibility {
	/// The mapping is accessible by userland.
	UserLocal,
	/// The mapping is kernel-only and VMS-local.
	KernelLocal,
	/// The mapping is kernel-only and global.
	KernelGlobal,
}

/// Valid RWX flag combinations
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RWX {
	R = 0b001,
	RW = 0b011,
	X = 0b100,
	RX = 0b101,
	RWX = 0b111,
}

/// A page that is either private or shared.
#[derive(Debug)]
pub enum PrivateOrShared {
	Private(PPN),
	Shared(SharedPPN),
}

/// Possible errors when adding a mapping
#[derive(Debug)]
pub enum AddError {
	/// The mapping overlaps with an existing mapping
	Overlaps,
	OutOfRange,
	AllocateError(AllocateError),
}

/// Possible errors when sharing a mapping
#[derive(Debug)]
pub enum ShareError {
	/// The mapping overlaps with an existing mapping
	Overlaps,
	OutOfRange,
	AllocateError(AllocateError),
	NoEntry,
}

impl From<AddError> for ShareError {
	fn from(error: AddError) -> Self {
		match error {
			AddError::Overlaps => Self::Overlaps,
			AddError::OutOfRange => Self::OutOfRange,
			AddError::AllocateError(e) => Self::AllocateError(e),
		}
	}
}

/// A trait that must be implemented by all VMSes.
pub trait VirtualMemorySystem
where
	Self: Sized,
{
	/// Create a new VMS.
	fn new() -> Result<Self, AllocateError>;

	/// Allocate the given amount of private pages and insert it as virtual memory at the
	/// given address.
	fn allocate(
		&mut self,
		virtual_address: Page,
		count: usize,
		rwx: RWX,
		accessibility: Accessibility,
	) -> Result<(), AddError>;

	/// Add a single page mapping to a specific VMS.
	fn add_to(
		&self,
		address: Page,
		map: Map,
		rwx: RWX,
		accessibility: Accessibility,
	) -> Result<(), AddError>;

	/// Add a single page mapping to the active VMS.
	fn add(address: Page, map: Map, rwx: RWX, accessibility: Accessibility)
		-> Result<(), AddError>;

	/// Map a range of pages. If the range of pages as well as the address are well aligned mega-
	/// and/or gigapages will be used.
	fn add_range(
		address: Page,
		map_range: MapRange,
		rwx: RWX,
		accessibility: Accessibility,
	) -> Result<(), AddError>;

	/// Remove a mapping and return the original PPN.
	///
	/// ## Returns
	///
	/// * `Ok(PPN)` if the mapping existed and was removed successfully.
	/// * `Err(())` if the mapping doesn't exist.
	fn remove(address: Page) -> Result<PrivateOrShared, ()>;

	/// Write the physical *addresses* from the start of the virtual address into the given slice.
	fn physical_addresses(address: Page, store: &mut [usize]) -> Result<(), ()>;

	/// Begin mapping a range of pages with PPNs passed from a function. Some of the PPNs may be
	/// used as tables.
	///
	/// This function never invokes the memory allocator directly and requires the passed PPNs to
	/// be identity mapped *and* not in any range of reserved memory.
	///
	/// It is intended only to be used by `crate::memory`. Use the other functions for regular
	/// allocations.
	fn allocate_pages<F>(f: F, address: Page, count: usize)
	where
		F: FnMut() -> PPN;

	/// Clear the identity maps.
	///
	/// This **must** only be called once at the end of early boot.
	fn clear_identity_maps();

	/// Returns the current memory map.
	fn current() -> Self;

	/// Map a page from the current VMS to this VMS.
	///
	/// This will mark private pages as shared.
	fn share(
		&self,
		self_address: Page,
		from_address: Page,
		rwx: RWX,
		accessibility: Accessibility,
	) -> Result<(), ShareError>;

	/// Activate this VMS, deactivating the current one.
	fn activate(&self);
}
