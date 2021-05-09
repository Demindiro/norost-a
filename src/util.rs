/// Error returned if there is not enough space in a buffer.
#[derive(Debug)]
pub struct BufferTooSmall;

/// Converts the given number into a `&str` with the given radix without allocation overhead by
/// writing the result into the given slice. It will print at least `digits` digits.
pub fn isize_to_string(buffer: &mut [u8], mut num: isize, radix: u8, digits: u8) -> Result<&str, BufferTooSmall> {
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
pub fn usize_to_string(buffer: &mut [u8], mut num: usize, radix: u8, digits: u8) -> Result<&str, BufferTooSmall> {
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
