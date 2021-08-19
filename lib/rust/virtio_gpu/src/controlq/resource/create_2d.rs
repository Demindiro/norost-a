use crate::ControlHeader;
use core::convert::TryFrom;
use core::fmt;
use simple_endian::u32le;

#[repr(C)]
pub struct Create2D {
	header: ControlHeader,
	resource_id: u32le,
	format: u32le,
	width: u32le,
	height: u32le,
}

impl Create2D {
	pub fn new(
		resource_id: u32,
		format: Format,
		width: u32,
		height: u32,
		fence: Option<u64>,
	) -> Self {
		Self {
			header: ControlHeader::new(ControlHeader::CMD_RESOURCE_CREATE_2D, fence),
			resource_id: resource_id.into(),
			format: u32::from(format).into(),
			width: width.into(),
			height: height.into(),
		}
	}
}

impl fmt::Debug for Create2D {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let mut d = f.debug_struct("resource::Create2D");
		d.field("header", &self.header);
		d.field("resource_id", &u32::from(self.resource_id));

		match Format::try_from(u32::from(self.format)) {
			Ok(f) => d.field("format", &f),
			Err(()) => d.field("format", &format_args!("0x{:x}", u32::from(self.format))),
		};

		d.field("width", &u32::from(self.resource_id));
		d.field("height", &u32::from(self.resource_id));
		d.finish()
	}
}

#[derive(Clone, Copy, Debug)]
#[repr(u32)]
#[non_exhaustive]
pub enum Format {
	BGRA8Unorm = 1,
	BGRX8Unorm = 2,
	ARGB8Unorm = 3,
	XRGB8Unorm = 4,
	RGBA8Unorm = 67,
	XBGR8Unorm = 68,
	ABGR8Unorm = 121,
	RGBX8Unorm = 134,
}

impl From<Format> for u32 {
	fn from(format: Format) -> u32 {
		format as u32
	}
}

impl TryFrom<u32> for Format {
	type Error = ();

	fn try_from(format: u32) -> Result<Self, Self::Error> {
		Ok(match format {
			1 => Self::BGRA8Unorm,
			2 => Self::BGRX8Unorm,
			3 => Self::ARGB8Unorm,
			4 => Self::XRGB8Unorm,
			67 => Self::RGBA8Unorm,
			68 => Self::XBGR8Unorm,
			121 => Self::ABGR8Unorm,
			134 => Self::RGBX8Unorm,
			_ => Err(())?,
		})
	}
}
