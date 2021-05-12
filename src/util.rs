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
			/// Creates a new `BigEndianU32` from a native-endian `u32`.
			pub fn new(value: $type) -> Self {
				Self(value.$to())
			}

			/// Gets the stored value as a native-endian `u32`
			pub fn get(self) -> $type {
				<$type>::$from(self.0)
			}

			/// Sets the stored value as a native-endian `u32`
			pub fn set(&mut self, value: $type) {
				self.0 = value.$to()
			}
		}
	};
	($name:ident($type:ty) ($from:ident, $to:ident)) => {
		gen_endian!($name, stringify!($type), $type, $from, $to);
	};
}

gen_endian!(BigEndianU32(u32)(from_be, to_be));
gen_endian!(BigEndianU64(u64)(from_be, to_be));
gen_endian!(LittleEndianU32(u32)(from_le, to_le));

/// Error returned if there is not enough space in a buffer.
#[derive(Debug)]
pub struct BufferTooSmall;

/// Converts the given number into a `&str` with the given radix without allocation overhead by
/// writing the result into the given slice. It will print at least `digits` digits.
pub fn isize_to_string(
	buffer: &mut [u8],
	mut num: isize,
	radix: u8,
	digits: u8,
) -> Result<&str, BufferTooSmall> {
	let radix = radix as isize;
	let mut digits = digits as isize;
	let len = buffer.len();
	let (buf, mut i) = if num < 0 {
		*buffer.get_mut(0).ok_or(BufferTooSmall)? = b'-';
		(&mut buffer[1..], len - 1)
	} else {
		// Use negative numbers as they have a larger range than positive numbers (+N + 1)
		num = -num;
		(&mut buffer[..], len)
	};
	while {
		i = i.checked_sub(1).ok_or(BufferTooSmall)?;
		let n = (-num % radix) as u8;
		buf[i] = if n < 10 { b'0' + n } else { b'a' - 10 + n };
		num /= radix;
		digits -= 1;
		digits > 0 || num != 0
	} {}
	Ok(core::str::from_utf8(&buffer[i..]).unwrap())
}

/// Converts the given number into a `&str` with the given radix without allocation overhead by
/// writing the result into the given slice.
pub fn usize_to_string(
	buffer: &mut [u8],
	mut num: usize,
	radix: u8,
	digits: u8,
) -> Result<&str, BufferTooSmall> {
	let radix = radix as usize;
	let mut digits = digits as isize;
	let mut i = buffer.len();
	while {
		i = i.checked_sub(1).ok_or(BufferTooSmall)?;
		let n = (num % radix) as u8;
		buffer[i] = if n < 10 { b'0' + n } else { b'a' - 10 + n };
		num /= radix;
		digits -= 1;
		digits > 0 || num != 0
	} {}
	Ok(core::str::from_utf8(&buffer[i..]).unwrap())
}

/// Converts a null-terminated C string to a Rust `str`.
///
/// ## SAFETY
///
/// The pointer must remain valid for as long as the returned `str`
pub unsafe fn cstr_to_str<'a>(cstr: *const u8) -> Result<&'a str, core::str::Utf8Error> {
	let mut len = 0;
	let mut buf = [0; 2];
	// SAFETY: the pointer remains withing a valid range
	while unsafe { *cstr.add(len) } != 0 {
		len += 1;
	}
	// SAFETY: The pointer and length are both valid
	let s = unsafe { slice::from_raw_parts(cstr, len) };
	core::str::from_utf8(s)
}
