//! # Task registry
//!
//! This is used as a way to identify tasks with human-readable names.

use super::*;
use crate::memory::reserved;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};

// TODO use a radix tree instead of a dumb list.

/// -1 means it's locked.
static REGISTRY_ENTRY_COUNT: AtomicUsize = AtomicUsize::new(0);
static REGISTRY: LOL = LOL([
	UnsafeCell::new(None),
	UnsafeCell::new(None),
	UnsafeCell::new(None),
	UnsafeCell::new(None),
	UnsafeCell::new(None),
	UnsafeCell::new(None),
	UnsafeCell::new(None),
	UnsafeCell::new(None),
	UnsafeCell::new(None),
	UnsafeCell::new(None),
	UnsafeCell::new(None),
	UnsafeCell::new(None),
	UnsafeCell::new(None),
	UnsafeCell::new(None),
	UnsafeCell::new(None),
	UnsafeCell::new(None),
]);

struct LOL([UnsafeCell<Option<Entry>>; 16]);

unsafe impl Sync for LOL {}

struct Entry {
	name_len: u8,
	name: [u8; 31],
	address: TaskID,
}

pub enum AddError {
	Occupied,
	NameTooLong,
	RegistryFull,
}

pub fn add(name: &[u8], address: TaskID) -> Result<(), AddError> {
	if name.len() > 31 {
		return Err(AddError::NameTooLong);
	}
	let len = lock();
	if len < REGISTRY.0.len() {
		let mut n = [0; 31];
		n[..name.len()].copy_from_slice(name);
		unsafe {
			REGISTRY.0[len].get().write(Some(Entry {
				name_len: name.len() as u8,
				name: n,
				address,
			}));
		}
		unlock(len + 1);
		Ok(())
	} else {
		unlock(len);
		Err(AddError::RegistryFull)
	}
}

pub fn get(name: &[u8]) -> Option<TaskID> {
	let len = lock();
	let e = REGISTRY.0[..len]
		.iter()
		.map(|e| unsafe { &*e.get() })
		.filter_map(Option::as_ref)
		.find(|e| &e.name[..usize::from(e.name_len)] == name)
		.map(|e| e.address);
	unlock(len);
	e
}

fn lock() -> usize {
	let mut len = REGISTRY_ENTRY_COUNT.load(Ordering::Relaxed);
	loop {
		match REGISTRY_ENTRY_COUNT.compare_exchange_weak(
			len,
			usize::MAX,
			Ordering::Acquire,
			Ordering::Relaxed,
		) {
			Ok(_) => break len,
			Err(v) => len = v,
		}
	}
}

fn unlock(len: usize) {
	REGISTRY_ENTRY_COUNT.store(len, Ordering::Release);
}
