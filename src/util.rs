/// Error returned if there is not enough space in a buffer.
#[derive(Debug)]
pub struct BufferTooSmall;

/// Converts the given number into a `&str` with radix 10 without allocation overhead by
/// writing the result into the given slice.
pub fn isize_to_string(buffer: &mut [u8], mut num: isize) -> Result<&str, BufferTooSmall> {
	let mut i = 0;
	if num < 0 {
		*buffer.get_mut(i).ok_or(BufferTooSmall)? = b'-';
		i += 1;
	} else {
		// Use negative numbers as they have a larger range than positive numbers (+N + 1)
		num = -num;
	}
	while {
		*buffer.get_mut(i).ok_or(BufferTooSmall)? = b'0' + -(num % 10) as u8;
		i += 1;
		num /= 10;
		num != 0
	} {}
	Ok(core::str::from_utf8(&buffer[..i]).unwrap())
}
