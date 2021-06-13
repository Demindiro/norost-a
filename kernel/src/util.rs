use core::slice;

/// This macro creates a big/little-endian wrapper for the given integer type.
macro_rules! gen_endian {
	($name:ident, $type_str:expr, $type:ty, $from:ident, $to:ident) => {
		#[doc = "A wrapper that encodes `"]
		#[doc = $type_str]
		#[doc = "`stored in big-endian format. This structure is used to prevent accidently"]
		#[doc = " reading `"]
		#[doc = $type_str]
		#[doc = "` in little-endian format."]
		#[derive(Clone, Copy)]
		#[repr(transparent)]
		pub struct $name($type);

		impl $name {
			/// Gets the stored value as a native-endian `u32`
			pub fn get(self) -> $type {
				<$type>::$from(self.0)
			}
		}
	};
	($name:ident($type:ty) ($from:ident, $to:ident)) => {
		gen_endian!($name, stringify!($type), $type, $from, $to);
	};
}

gen_endian!(BigEndianU32(u32)(from_be, to_be));
gen_endian!(BigEndianU64(u64)(from_be, to_be));

/// Error returned if there is not enough space in a buffer.
#[derive(Debug)]
pub struct BufferTooSmall;

/// Converts a null-terminated C string to a Rust `str`.
///
/// ## SAFETY
///
/// The pointer must remain valid for as long as the returned `str`
pub unsafe fn cstr_to_str<'a>(cstr: *const u8) -> Result<&'a str, core::str::Utf8Error> {
	let mut len = 0;
	// SAFETY: the pointer remains withing a valid range
	while *cstr.add(len) != 0 {
		len += 1;
	}
	// SAFETY: The pointer and length are both valid
	let s = slice::from_raw_parts(cstr, len);
	core::str::from_utf8(s)
}
