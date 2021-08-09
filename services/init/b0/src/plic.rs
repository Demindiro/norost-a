//! # RISC-V PLIC stub
//!
//! Implemented based on information from
//! https://github.com/riscv/riscv-plic-spec/blob/master/riscv-plic.adoc

use core::convert::TryFrom;
use core::mem;
use core::num::NonZeroU16;
use core::ptr;
use core::ptr::NonNull;

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
pub enum EnableError {
	InvalidSource,
	InvalidContext,
}

impl PLIC {

	// The offsets are expressed in _u32_s!
	const OFFSET_PRIORITY: usize = 0x0000 / mem::size_of::<u32>();
	const OFFSET_PENDING_BITS: usize = 0x1000 / mem::size_of::<u32>();
	const OFFSET_ENABLE_BITS: usize = 0x2000 / mem::size_of::<u32>();
	const OFFSET_PRIORITY_THRESHOLDS: usize = 0x20_0000 / mem::size_of::<u32>();

	const WIDTH_CONTEXT_ENABLE_BITS: usize = 0x80 / mem::size_of::<u32>();
	const WIDTH_CONTEXT_PRIORITY_THRESHOLDS: usize = 0x1000 / mem::size_of::<u32>();

	/// Setup a new PLIC controller.
	///
	/// # Safety
	///
	/// The address must actually point to a PLIC controller MMIO map. It may not be
	/// in use by anything else either yet.
	pub unsafe fn new(base_address: NonNull<u32>) -> Self {
		Self { base_address }
	}

	/// Set the priority of an interrupt source.
	pub fn set_priority(&mut self, source: NonZeroU16, priority: u32) -> Result<(), InvalidSource> {
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
	pub fn check_pending(&mut self, source: NonZeroU16) -> Result<bool, InvalidSource> {
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
	pub fn enable(&mut self, context: u16, source: NonZeroU16, enable: bool) -> Result<(), EnableError> {
		Self::context_in_range(context).map_err(|_| EnableError::InvalidContext)?;
		let (offt, bit) = Self::split_source(source).map_err(|_| EnableError::InvalidSource)?;
		unsafe {
			let addr = self
				.base_address
				.as_ptr()
				.add(Self::OFFSET_ENABLE_BITS)
				.add(usize::from(context) * Self::WIDTH_CONTEXT_ENABLE_BITS)
				.add(offt);
			let val = ptr::read_volatile(addr);
			ptr::write_volatile(addr, (val & !(1 << bit)) | (u32::from(enable) << bit));
			Ok(())
		}
	}

	/// Set the priority threshold of a context
	pub fn set_priority_threshold(&mut self, context: u16, threshold: u32) -> Result<(), InvalidContext> {
		Self::context_in_range(context)?;
		unsafe {
			let addr = self
				.base_address
				.as_ptr()
				.add(Self::OFFSET_PRIORITY_THRESHOLDS)
				.add(usize::from(context) * Self::WIDTH_CONTEXT_PRIORITY_THRESHOLDS);
			ptr::write_volatile(addr, threshold);
			Ok(())
		}
	}

	// TODO figure out how this claim/completion thing works exactly. The documentation suggests
	// it is set by the PLIC automatically, but then what is it for?

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
		Self::source_in_range(source)
			.map(|()| (usize::from(source.get() / 32), u8::try_from(source.get() & 31).unwrap()))
	}
}