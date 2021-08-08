use crate::ffi;
use core::ptr::NonNull;
use dux::mem::*;

#[repr(C)]
struct ReservePagesStatus {
	address: Option<NonNull<kernel::Page>>,
	status: i8,
}

#[no_mangle]
extern "C" fn dux_reserve_pages(address: Option<NonNull<kernel::Page>>, count: usize) -> ReservePagesStatus {
	let addr = match address {
		Some(addr) => match dux::Page::new(addr) {
			Ok(addr) => Some(addr),
			Err(dux::page::Unaligned) => return ReservePagesStatus {
				address: None,
				status: -2,
			},
		}
		None => None,
	};
	match reserve_range(addr, count) {
		Ok(addr) => ReservePagesStatus {
			address: Some(addr.as_non_null_ptr()),
			status: 0,
		},
		Err(ReserveError::NoSpace) => ReservePagesStatus {
			address: None,
			status: -1,
		},
		Err(ReserveError::NoMemory) => ReservePagesStatus {
			address: None,
			status: -3,
		},
	}
}

#[no_mangle]
extern "C" fn dux_unreserve_pages(address: *mut kernel::Page, count: usize) -> ffi::c_int {
	match NonNull::new(address).and_then(|addr| dux::Page::new(addr).ok()) {
		Some(addr) => match unreserve_range(addr, count) {
			Ok(()) => 0,
			Err(UnreserveError::InvalidAddress) => -2,
			Err(UnreserveError::SizeTooLarge) => -3,
		}
		None => -1,
	}
}
