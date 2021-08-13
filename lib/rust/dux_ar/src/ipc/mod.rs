mod list;

use crate::ffi;
use core::ptr::NonNull;
use dux::ipc::*;

#[no_mangle]
extern "C" fn dux_add_free_range(
	address: Option<NonNull<kernel::Page>>,
	count: usize,
) -> ffi::c_int {
	match address.and_then(|addr| dux::Page::new(addr).ok()) {
		Some(addr) => match add_free_range(addr, count) {
			Ok(()) => 0,
			Err(()) => -2,
		},
		None => -1,
	}
}

#[no_mangle]
unsafe extern "C" fn dux_reserve_transmit_entry(packet: &mut *mut kernel::ipc::Packet) -> u16 {
	let (slot, pkt) = transmit().into_raw();
	*packet = pkt as *mut _;
	slot
}

#[no_mangle]
unsafe extern "C" fn dux_submit_transmit_entry(slot: u16) {
	TransmitLock::from_raw(slot);
}

#[no_mangle]
extern "C" fn dux_get_received_entry(packet: &mut *const kernel::ipc::Packet) -> u16 {
	let (slot, pkt) = receive().into_raw();
	*packet = pkt as *const _;
	slot
}

#[no_mangle]
unsafe extern "C" fn dux_pop_received_entry(slot: u16) {
	ReceivedLock::from_raw(slot);
}

#[no_mangle]
unsafe extern "C" fn dux_defer_received_entry(slot: u16) {
	ReceivedLock::from_raw(slot).defer();
}
