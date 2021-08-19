use crate::ControlHeader;
use simple_endian::u32le;

#[repr(C)]
pub struct GetEDID {
	header: ControlHeader,
	scanout: u32le,
	_padding: u32le,
}

#[repr(C)]
struct ResponseEDID {
	header: ControlHeader,
	size: u32le,
	_padding: u32le,
	edid: [u8; 1024],
}
