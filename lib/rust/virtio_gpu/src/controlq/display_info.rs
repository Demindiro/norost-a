use super::*;

const MAX_SCANOUTS: u32 = 16;

#[repr(C)]
pub struct ResponseDisplayInfo {
	header: ControlHeader,
	pmodes: [DisplayOne; MAX_SCANOUTS as usize],
}

#[repr(C)]
struct DisplayOne {
	rect: Rect,
	enabled: u32le,
	flags: u32le,
}
