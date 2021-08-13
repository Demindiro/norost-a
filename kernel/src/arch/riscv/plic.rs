//! # PLIC handling module
//!
//! Implemented based on information from
//! https://github.com/riscv/riscv-plic-spec/blob/master/riscv-plic.adoc

use crate::arch::{MapRange, VMS};
use crate::memory::reserved;
use crate::task::Address;
use crate::util::OnceCell;
use core::convert::TryFrom;
use core::mem;
use core::num::NonZeroU16;
use core::ptr;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};

/// The total amount of available interrupt sources.
///
/// This *excludes* the non-existent interrupt 0.
static TOTAL_SOURCES: OnceCell<u16> = OnceCell::new(0);

/// The list of reserved interrupts.
///
/// A value of usize::MAX indicates the slot is free.
///
/// Note that it is offset by one, i.e. slot 0 refers to interrupt 1.
// Using 0 as using usize::MAX would cause RESERVATIONS to be stored in the
// ELF file, bloating it.
#[export_name = "plic_reservations"]
static RESERVATIONS: [AtomicUsize; 1023] = [STUPIDITY; 1023];

const STUPIDITY: AtomicUsize = AtomicUsize::new(0);

/// A PLIC abstraction with the base address set to where the PLIC is supposed to be mapped.
const PLIC: PLIC = PLIC {
	base_address: reserved::PLIC.start.as_non_null_ptr().cast(),
};

#[derive(Debug)]
pub enum ReserveError {
	Occupied,
	NonExistent,
}

/// Set the interrupt controller.
///
/// # Safety
///
/// This may only be called once.
///
/// It must point to a PLIC MMIO area.
pub unsafe fn set_controller(range: MapRange, max_devices: u16) {
	TOTAL_SOURCES.set(max_devices);
	use crate::arch::vms::{Accessibility, VirtualMemorySystem, RWX};
	VMS::add_range(
		reserved::PLIC.start,
		range,
		RWX::RW,
		Accessibility::KernelGlobal,
	)
	.unwrap();
	// Set up the reservations now.
	for e in RESERVATIONS.iter() {
		e.store(usize::MAX, Ordering::Relaxed);
	}
}

/// Reserve an interrupt source
pub fn reserve(source: u16, address: Address) -> Result<(), ReserveError> {
	let source = source.checked_sub(1).ok_or(ReserveError::NonExistent)?;
	(source < *TOTAL_SOURCES)
		.then(|| ())
		.ok_or(ReserveError::NonExistent)?;
	let entry = &RESERVATIONS[usize::from(source)];
	entry
		.compare_exchange(
			usize::MAX,
			address.into(),
			Ordering::Relaxed,
			Ordering::Relaxed,
		)
		.map_err(|_| ReserveError::Occupied)?;

	// The PLIC's behaviour should match that of SiFive's PLIC
	// https://static.dev.sifive.com/U54-MC-RVCoreIP.pdf
	// Presumably, since we're running on hart 0 (the only hart), we need to
	// enable the interrupt in context 0x1 (S-mode).

	let context = 1; // TODO this should be done for all available harts
	let source = NonZeroU16::new(source + 1).unwrap();

	PLIC.enable(context, source, true).unwrap();
	PLIC.set_priority(source, 1).unwrap();
	PLIC.set_priority_threshold(context, 0).unwrap();

	Ok(())
}

/// A RISC-V Platform Level Interrupt Controller. This must be set up to receive
/// interrupts at all.
pub struct PLIC {
	base_address: NonNull<u32>,
}

/// Error returned if a source ID isn't valid.
#[derive(Debug)]
pub struct InvalidSource;

/// Error returned if a context ID isn't valid.
#[derive(Debug)]
pub struct InvalidContext;

#[derive(Debug)]
pub enum InvalidContextOrSource {
	InvalidSource,
	InvalidContext,
}

impl From<InvalidSource> for InvalidContextOrSource {
	fn from(_: InvalidSource) -> Self {
		Self::InvalidSource
	}
}

impl From<InvalidContext> for InvalidContextOrSource {
	fn from(_: InvalidContext) -> Self {
		Self::InvalidContext
	}
}

impl PLIC {
	// The offsets are expressed in _u32_s!
	const OFFSET_PRIORITY: usize = 0x0000 / mem::size_of::<u32>();
	const OFFSET_PENDING_BITS: usize = 0x1000 / mem::size_of::<u32>();
	const OFFSET_ENABLE_BITS: usize = 0x2000 / mem::size_of::<u32>();
	const OFFSET_PRIORITY_THRESHOLDS: usize = 0x20_0000 / mem::size_of::<u32>();
	const OFFSET_CLAIM_COMPLETE: usize = 0x20_0004 / mem::size_of::<u32>();

	const STRIDE_ENABLE_BITS: usize = 0x80 / mem::size_of::<u32>();
	const STRIDE_PRIORITY_THRESHOLDS: usize = 0x1000 / mem::size_of::<u32>();
	const STRIDE_CLAIM_COMPLETE: usize = 0x1000 / mem::size_of::<u32>();

	/// Set the priority of an interrupt source.
	pub fn set_priority(&self, source: NonZeroU16, priority: u32) -> Result<(), InvalidSource> {
		Self::source_in_range(source)?;
		unsafe {
			let addr = self
				.base_address
				.as_ptr()
				.add(Self::OFFSET_PRIORITY)
				.add(source.get().into());
			ptr::write_volatile(addr, priority);
			Ok(())
		}
	}

	/// Check if an interrupt is pending.
	#[allow(dead_code)]
	pub fn check_pending(&self, source: NonZeroU16) -> Result<bool, InvalidSource> {
		let (offt, bit) = Self::split_source(source)?;
		unsafe {
			let addr = self
				.base_address
				.as_ptr()
				.add(Self::OFFSET_PENDING_BITS)
				.add(offt);
			Ok(ptr::read_volatile(addr) & (1 << bit) > 0)
		}
	}

	/// Enable or disable an interrupt.
	pub fn enable(
		&self,
		context: u16,
		source: NonZeroU16,
		enable: bool,
	) -> Result<(), InvalidContextOrSource> {
		Self::context_in_range(context)?;
		let (offt, bit) = Self::split_source(source)?;
		unsafe {
			let addr = self
				.base_address
				.as_ptr()
				.add(Self::OFFSET_ENABLE_BITS)
				.add(usize::from(context) * Self::STRIDE_ENABLE_BITS)
				.add(offt);
			let val = ptr::read_volatile(addr);
			ptr::write_volatile(addr, (val & !(1 << bit)) | (u32::from(enable) << bit));
			Ok(())
		}
	}

	/// Set the priority threshold of a context
	pub fn set_priority_threshold(
		&self,
		context: u16,
		threshold: u32,
	) -> Result<(), InvalidContext> {
		Self::context_in_range(context)?;
		unsafe {
			let addr = self
				.base_address
				.as_ptr()
				.add(Self::OFFSET_PRIORITY_THRESHOLDS)
				.add(usize::from(context) * Self::STRIDE_PRIORITY_THRESHOLDS);
			ptr::write_volatile(addr, threshold);
			Ok(())
		}
	}

	/// Claim an interrupt.
	pub fn claim(&self, context: u16) -> Result<Option<NonZeroU16>, InvalidContext> {
		Self::context_in_range(context)?;
		unsafe {
			let addr = self
				.base_address
				.as_ptr()
				.add(Self::OFFSET_CLAIM_COMPLETE)
				.add(usize::from(context) * Self::STRIDE_CLAIM_COMPLETE);
			let source = ptr::read_volatile(addr);
			assert!(source < 1024, "source is out of range");
			Ok(NonZeroU16::new(u16::try_from(source).unwrap()))
		}
	}

	/// Mark an interrupt as completed.
	pub fn complete(&self, context: u16, source: NonZeroU16) -> Result<(), InvalidContextOrSource> {
		Self::context_in_range(context)?;
		Self::source_in_range(source)?;
		unsafe {
			let addr = self
				.base_address
				.as_ptr()
				.add(Self::OFFSET_CLAIM_COMPLETE)
				.add(usize::from(context) * Self::STRIDE_CLAIM_COMPLETE);
			ptr::write_volatile(addr, source.get().into());
			Ok(())
		}
	}

	/// Ensure the source is in range, i.e. it is below 1024.
	fn source_in_range(source: NonZeroU16) -> Result<(), InvalidSource> {
		(source.get() < 1024).then(|| ()).ok_or(InvalidSource)
	}

	/// Ensure the context is in range, i.e. it is below 15872 (`0x3e00`).
	fn context_in_range(context: u16) -> Result<(), InvalidContext> {
		(context < 0x3e00).then(|| ()).ok_or(InvalidContext)
	}

	/// Split a source address in an address offset and a bit offset
	fn split_source(source: NonZeroU16) -> Result<(usize, u8), InvalidSource> {
		Self::source_in_range(source).map(|()| {
			(
				usize::from(source.get() / 32),
				u8::try_from(source.get() & 31).unwrap(),
			)
		})
	}
}
