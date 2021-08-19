use super::*;

#[repr(C)]
struct ResourceDetachBacking {
	header: ControlHeader,
	resource_id: u32le,
	_padding: u32le,
}
