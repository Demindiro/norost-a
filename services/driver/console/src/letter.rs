//! Table of letter bitmaps.

use crate::RGBA8;

/// Bitmap of letters stolen from https://forum.osdev.org/viewtopic.php?f=2&t=20833
///
/// The bitmap has been manually compressed to be an actual bitmap instead of bytemap. It also
/// has been formatted such that each letter is contiguous in memory which is a lot easier to
/// manage.
const LETTERS: [u8; Letter::SIZE * 256] = *include_bytes!("../font.bitmap");

/// A 8x12 letter
pub struct Letter {
	letter: u8,
}

impl Letter {
	pub const WIDTH: usize = 9;
	pub const HEIGHT: usize = 16;
	pub const BITS: usize = Self::WIDTH * Self::HEIGHT;
	pub const SIZE: usize = Self::BITS / 8;

	/// Check if a bit is on or off.
	///
	/// # Panics
	///
	/// If the index is out of range.
	#[inline]
	fn get(&self, x: usize, y: usize) -> bool {
		assert!(x < Self::WIDTH);
		assert!(y < Self::HEIGHT);
		let i = usize::from(self.letter) * Self::BITS + y * Self::WIDTH + x;
		LETTERS[i / 8] & (1 << (i % 8)) > 0
	}

	/// Copy a letter to the given buffer with the given foreground and background color.
	pub(crate) fn copy(
		&self,
		x: usize,
		y: usize,
		buffer: &mut [RGBA8],
		w: usize,
		h: usize,
		fg: RGBA8,
		bg: RGBA8,
	) {
		let _ = h;
		assert!(x + w * y < buffer.len());
		for (ly, wy) in (0..Self::HEIGHT).zip(y..y + Self::HEIGHT) {
			for (lx, wx) in (0..Self::WIDTH).zip(x..x + Self::WIDTH) {
				buffer[wx + wy * w] = self.get(lx, ly).then(|| fg).unwrap_or(bg);
			}
		}
	}
}

#[derive(Debug)]
pub struct OutOfBounds;

/// Return a specific letter.
#[inline(always)]
pub fn get(letter: u8) -> Letter {
	Letter { letter }
}
