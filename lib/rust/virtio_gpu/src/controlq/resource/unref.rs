use super::*;

#[repr(C)]
struct ResourceUnreference {
	header: ControlHeader,
	resource_id: u32le,
	padding: u32le,
}
