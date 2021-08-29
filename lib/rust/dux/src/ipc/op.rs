use core::mem;

#[derive(Clone, Copy, Debug)]
pub enum Op {
	Open = 0,
	Close = 1,
	Map = 2,
	Flush = 3,
	Stat = 4,
	List = 5,
	Create = 6,
	Destroy = 7,

	Terminate = 15,

	Ok = 31,
	Exists = 30,
	NoPermission = 29,
	Unavailable = 28,
}

impl Op {
	pub fn from_flags(flags: u16) -> Result<Self, i8> {
		Ok(match flags >> 11 {
			0 => Self::Open,
			1 => Self::Close,
			2 => Self::Map,
			3 => Self::Flush,
			4 => Self::Stat,
			5 => Self::List,
			6 => Self::Create,
			7 => Self::Destroy,

			15 => Self::Terminate,

			31 => Self::Ok,
			30 => Self::Exists,
			29 => Self::NoPermission,
			28 => Self::Unavailable,

			flags => return Err(flags as i8),
		})
	}

	pub fn to_flags(self) -> u16 {
		(self as u16) << 11
	}
}
