use crate::ffi;
use core::ptr;
use dux::ipc::list::*;

#[repr(C)]
struct Entry {
	pub uuid: kernel::ipc::UUID,
	pub size: u64,
	pub name: *const u8,
	pub name_length: u16,
}

#[no_mangle]
extern "C" fn dux_ipc_list_get(list: &List, index: usize, entry: &mut Entry) -> ffi::c_int {
	list.get(index)
		.map(|e| {
			entry.uuid = e.uuid;
			entry.size = e.size;
			// The pointer should be valid for as long as the list remains unmodified.
			entry.name = e.name.map(|n| n.as_ptr()).unwrap_or_else(ptr::null);
			// The name shouldn't ever be larger than 2^16.
			entry.name_length = e.name.map(|n| n.len()).unwrap_or(0) as u16;
			0
		})
		.unwrap_or(-1)
}
