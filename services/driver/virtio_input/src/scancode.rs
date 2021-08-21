use core::num::NonZeroU8;

const DEFAULT: &str = include_str!("../scancode_sets/evdev.kbd");

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Key {
	None,
	Char(char),
	LShift,
	RShift,
	LCtrl,
	RCtrl,
	LSuper,
	RSuper,
	ArrowDown,
	ArrowUp,
	ArrowLeft,
	ArrowRight,
	F(u8),
	Tab,
	Enter,
	Capslock,
	Escape,
	Backspace,
	Alt,
	AltGr,
	Space,
	Pause,
}

impl Key {
	fn from_name(name: &str) -> Result<Self, InvalidKeyName> {
		Ok(match name {
			"alt" => Self::Alt,
			"altgr" => Self::AltGr,
			"arrowd" => Self::ArrowDown,
			"arrowl" => Self::ArrowLeft,
			"arrowr" => Self::ArrowRight,
			"arrowu" => Self::ArrowUp,
			"backspace" => Self::Backspace,
			"capslock" => Self::Capslock,
			"enter" => Self::Enter,
			"escape" => Self::Escape,
			"pause" => Self::Pause,
			"lsuper" => Self::LSuper,
			"rsuper" => Self::RSuper,
			"lctrl" => Self::LCtrl,
			"rctrl" => Self::RCtrl,
			"lshift" => Self::LShift,
			"rshift" => Self::RShift,
			"space" => Self::Space,
			"tab" => Self::Tab,
			_ if name.chars().count() == 1 => Self::Char(name.chars().next().unwrap()),
			_ if name.chars().next() == Some('f') => {
				Self::F(u8::from_str_radix(&name[1..], 10).unwrap())
			}
			_ => todo!("{:?}", name),
		})
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Modifiers(u8);

impl Modifiers {
	const CAPS: u8 = 0x1;

	fn from_strs<'a>(strs: impl Iterator<Item = &'a str>) -> Self {
		let mut n = 0;
		for s in strs {
			match s {
				"caps" => n |= Self::CAPS,
				s => todo!("{:?}", s),
			}
		}
		Self(n)
	}

	pub fn new() -> Self {
		Self(0)
	}

	pub fn caps(&self) -> bool {
		self.0 & Self::CAPS > 0
	}

	pub fn set_caps(&mut self, enable: bool) {
		self.0 &= !Self::CAPS;
		self.0 |= Self::CAPS * u8::from(enable)
	}
}

pub struct ScanCodes {
	list: [Option<((Modifiers, NonZeroU8), Key)>; 256],
	len: usize,
}

impl ScanCodes {
	pub fn from_file(file: &str) -> Result<Self, ParseError> {
		let mut list = [None; 256];
		let mut len = 0;
		for (w, line) in list.iter_mut().zip(
			file.lines()
				.filter(|l| l != &"" && l.chars().next() != Some('#')),
		) {
			let mut split = line
				.rsplit(|c: char| c.is_ascii_whitespace())
				.filter(|l| l != &"");
			let name = split.next().expect("no name");
			let code = u8::from_str_radix(split.next().expect("no code"), 16);
			let code = NonZeroU8::new(code.expect("invalid code")).expect("invalid code");
			let mods = Modifiers::from_strs(split);
			*w = Some(((mods, code), Key::from_name(name).unwrap()));
			len += 1;
		}
		// Sort for fast lookup
		list[..len].sort_unstable();
		// Check for duplicates
		let mut last = list[0];
		for e in list[..len].iter().skip(1).copied() {
			assert_ne!(e, last, "duplicate entry");
			last = e;
		}

		Ok(Self { list, len })
	}

	pub fn get(&self, modifiers: Modifiers, code: NonZeroU8) -> Option<Key> {
		self.list[..self.len]
			.binary_search_by(|k| k.unwrap().0.cmp(&(modifiers, code)))
			.ok()
			.map(|i| self.list[i].unwrap().1)
	}
}

#[derive(Debug)]
pub enum ParseError {}

#[derive(Debug)]
pub struct InvalidKeyName;

pub fn default() -> ScanCodes {
	ScanCodes::from_file(DEFAULT).expect("failed to parse default scan codes")
}
