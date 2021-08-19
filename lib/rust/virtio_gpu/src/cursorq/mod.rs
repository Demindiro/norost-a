use crate::ControlHeader;
use simple_endian::u32le;

#[repr(C)]
pub struct CursorPosition {
	pub scanout_id: u32le,
	pub x: u32le,
	pub y: u32le,
	_padding: u32le,
}

#[repr(C)]
pub struct UpdateCursor {
	header: ControlHeader,
	pub position: CursorPosition,
	pub resource_id: u32le,
	pub hot_x: u32le,
	pub hot_y: u32le,
	_padding: u32le,
}
