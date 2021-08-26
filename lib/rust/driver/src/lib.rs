//! # Helper library for drivers

#![no_std]
#![feature(optimize_attribute)]

use core::fmt;
use core::num;
use core::str;

macro_rules! derive {
	(@INTERNAL impl to_args($self:ident, $buf:ident, $alloc:ident, $add_arg:ident) for $name:ident $code:tt) => {
		impl $name {
			pub fn to_args<'a, F, G>(&$self, $buf: &'a mut [u8], $alloc: F, $add_arg: G) -> Result<&'a mut [u8], OutOfMemory>
			where
				F: FnMut(&'a mut [u8], usize) -> Result<(&'a mut [u8], &'a mut [u8]), OutOfMemory>,
				G: FnMut(&'a str) -> Result<(), OutOfMemory>,
			{
				$code
			}
		}
	};
	(@INTERNAL impl from_args($buf:ident[$len:literal]) for $name:ident $tuple:expr) => {
		impl $name {
			pub fn from_args<'a, I>(mut arguments: I) -> Result<Self, ParseError<'a>>
			where
				I: Iterator<Item = &'a [u8]>,
			{
				let mut $buf = [(&[][..], "", 0); $len];
				from(&mut arguments, stringify!($name), &mut $buf[..])?;
				Ok($tuple)
			}
		}
	};
	(@INTERNAL impl fmt::Debug($self:ident) for $name:ident { $list:expr }) => {
		impl fmt::Debug for $name {
			#[optimize(size)]
			fn fmt(&$self, f: &mut fmt::Formatter) -> fmt::Result {
				fmt(f, stringify!($name), &$list)
			}
		}
	};
	($name:ident $arg:literal $a:ident) => {
		#[derive(Clone, Copy)]
		pub struct $name {
			pub $a: u128,
		}

		impl $name {
			pub const CMD_ARG: &'static str = $arg;

			#[inline(always)]
			pub const fn new($a: u128) -> Self {
				Self { $a }
			}
		}

		derive!(@INTERNAL impl to_args(self, buffer, alloc, add_argument) for $name {
			to(concat!("--", $arg), buffer, alloc, add_argument, &[self.$a])
		});

		derive!(@INTERNAL impl from_args(buffer[1]) for $name {
			Self { $a: buffer[0].2 }
		});

		derive!(@INTERNAL impl fmt::Debug(self) for $name {
			[(stringify!($a), self.$a)]
		});
	};
	($name:ident $arg:literal $a:ident $b:ident) => {
		#[derive(Clone, Copy)]
		pub struct $name {
			pub $a: u128,
			pub $b: u128,
		}

		impl $name {
			pub const CMD_ARG: &'static str = $arg;

			#[inline(always)]
			pub const fn new($a: u128, $b: u128) -> Self {
				Self { $a, $b }
			}
		}

		derive!(@INTERNAL impl to_args(self, buffer, alloc, add_argument) for $name {
			to(concat!("--", $arg), buffer, alloc, add_argument, &[self.$a, self.$b])
		});

		derive!(@INTERNAL impl from_args(buffer[2]) for $name {
			Self { $a: buffer[0].2, $b: buffer[1].2 }
		});

		derive!(@INTERNAL impl fmt::Debug(self) for $name {
			[(stringify!($a), self.$a), (stringify!($b), self.$b)]
		});
	};
	($name:ident $arg:literal $a:ident $b:ident $c:ident) => {
		#[derive(Clone, Copy)]
		pub struct $name {
			pub $a: u128,
			pub $b: u128,
			pub $c: u128,
		}

		impl $name {
			pub const CMD_ARG: &'static str = $arg;

			#[inline(always)]
			pub const fn new($a: u128, $b: u128, $c: u128) -> Self {
				Self { $a, $b, $c }
			}
		}

		derive!(@INTERNAL impl to_args(self, buffer, alloc, add_argument) for $name {
			to(concat!("--", $arg), buffer, alloc, add_argument, &[self.$a, self.$b, self.$c])
		});

		derive!(@INTERNAL impl from_args(buffer[3]) for $name {
			Self { $a: buffer[0].2, $b: buffer[1].2, $c: buffer[2].2 }
		});

		derive!(@INTERNAL impl fmt::Debug(self) for $name {
			[(stringify!($a), self.$a), (stringify!($b), self.$b), (stringify!($c), self.$c)]
		});
	};
	($name:ident $arg:literal $a:ident $b:ident $c:ident $d:ident $e:ident) => {
		#[derive(Clone, Copy)]
		pub struct $name {
			pub $a: u128,
			pub $b: u128,
			pub $c: u128,
			pub $d: u128,
			pub $e: u128,
		}

		impl $name {
			pub const CMD_ARG: &'static str = $arg;

			#[inline(always)]
			pub const fn new($a: u128, $b: u128, $c: u128, $d: u128, $e: u128) -> Self {
				Self { $a, $b, $c, $d, $e }
			}
		}

		derive!(@INTERNAL impl to_args(self, buffer, alloc, add_argument) for $name {
			to(concat!("--", $arg), buffer, alloc, add_argument, &[self.$a, self.$b, self.$c, self.$d, self.$e])
		});

		derive!(@INTERNAL impl from_args(buffer[5]) for $name {
			Self {
				$a: buffer[0].2,
				$b: buffer[1].2,
				$c: buffer[2].2,
				$d: buffer[3].2,
				$e: buffer[4].2,
			}
		});

		derive!(@INTERNAL impl fmt::Debug(self) for $name {
			[
				(stringify!($a), self.$a),
				(stringify!($b), self.$b),
				(stringify!($c), self.$c),
				(stringify!($d), self.$d),
				(stringify!($e), self.$e),
			]
		});
	};
}

#[derive(Debug)]
pub struct OutOfMemory;

#[allow(dead_code)]
fn fmt(f: &mut fmt::Formatter, name: &str, list: &[(&str, u128)]) -> fmt::Result {
	let mut ds = f.debug_struct(name);
	list.iter().copied().for_each(|(n, v)| {
		ds.field(n, &format_args!("0x{:x}", v));
	});
	ds.finish()
}

// TODO use generic instead of u8 slice as buffer
fn to<'a, F, G>(
	name: &'a str,
	mut alloc_buffer: &'a mut [u8],
	mut alloc: F,
	mut add_argument: G,
	args: &[u128],
) -> Result<&'a mut [u8], OutOfMemory>
where
	F: FnMut(&'a mut [u8], usize) -> Result<(&'a mut [u8], &'a mut [u8]), OutOfMemory>,
	G: FnMut(&'a str) -> Result<(), OutOfMemory>,
{
	fn len_hex(mut num: u128) -> usize {
		let mut i = 0;
		while {
			i += 1;
			num >>= 4;
			num != 0
		} {}
		i
	}

	fn fmt_hex(buf: &mut [u8], mut num: u128) -> &str {
		let mut i = buf.len() - 1;
		while {
			let d = (num % 16) as u8;
			buf[i] = (d < 10).then(|| b'0').unwrap_or(b'a' - 10) + d;
			num /= 16;
			i -= 1;
			num != 0
		} {}
		core::str::from_utf8(buf).unwrap()
	}

	add_argument(name)?;
	for a in args.iter().copied() {
		let (b, r) = alloc(alloc_buffer, len_hex(a))?;
		add_argument(fmt_hex(b, a))?;
		alloc_buffer = r;
	}

	Ok(alloc_buffer)
}

fn from<'a, I>(
	args: &mut I,
	name: &'static str,
	buf: &mut [(&'a [u8], &'a str, u128)],
) -> Result<(), ParseError<'a>>
where
	I: Iterator<Item = &'a [u8]>,
{
	for e in buf.iter_mut() {
		e.0 = args.next().ok_or(ParseError::MissingArgument(name))?;
	}
	for e in buf.iter_mut() {
		e.1 = str::from_utf8(e.0).map_err(ParseError::Utf8Error)?;
	}
	for e in buf.iter_mut() {
		e.2 = u128::from_str_radix(e.1, 16).map_err(ParseError::ParseIntError)?;
	}
	Ok(())
}

derive!(Reg "reg" address size);
derive!(Range "range" child_address address size);
derive!(InterruptMap "interrupt-map" child_address child_interrupt parent parent_address parent_interrupt);
derive!(InterruptMapMask "interrupt-map-mask" child_address child_interrupt);
derive!(Pci "pci" child_address address size);
derive!(PciInterrupt "pci-interrupt" line pin);
derive!(BarMmio "bar-mmio" index address size);
derive!(BarIo "bar-io" index address size);

#[derive(Debug)]
#[non_exhaustive]
pub enum Arg<'a> {
	#[cfg(any(feature = "parse-reg", feature = "to-reg"))]
	Reg(Reg),
	#[cfg(any(feature = "parse-range", feature = "to-range"))]
	Range(Range),
	#[cfg(any(feature = "parse-interrupt-map", feature = "to-interrupt-map"))]
	InterruptMap(InterruptMap),
	#[cfg(any(
		feature = "parse-interrupt-map-mask",
		feature = "to-interrupt-map-mask"
	))]
	InterruptMapMask(InterruptMapMask),
	#[cfg(any(feature = "parse-pci", feature = "to-pci"))]
	Pci(Pci),
	#[cfg(any(feature = "parse-pci-interrupt", feature = "to-pci-interrupt"))]
	PciInterrupt(PciInterrupt),
	#[cfg(any(feature = "parse-bar-io", feature = "to-bar-io"))]
	BarIo(BarIo),
	#[cfg(any(feature = "parse-bar-mmio", feature = "to-bar-mmio"))]
	BarMmio(BarMmio),
	Other(&'a [u8]),
}

impl Arg<'_> {
	#[optimize(size)]
	pub fn cmd_arg(&self) -> Result<&str, &[u8]> {
		Ok(match self {
			#[cfg(any(feature = "parse-reg", feature = "to-reg"))]
			Self::Reg(_) => Reg::CMD_ARG,
			#[cfg(any(feature = "parse-range", feature = "to-range"))]
			Self::Range(_) => Range::CMD_ARG,
			#[cfg(any(feature = "parse-interrupt-map", feature = "to-interrupt-map"))]
			Self::InterruptMap(_) => InterruptMap::CMD_ARG,
			#[cfg(any(
				feature = "parse-interrupt-map-mask",
				feature = "to-interrupt-map-mask"
			))]
			Self::InterruptMapMask(_) => InterruptMapMask::CMD_ARG,
			#[cfg(any(feature = "parse-pci", feature = "to-pci"))]
			Self::Pci(_) => Pci::CMD_ARG,
			#[cfg(any(feature = "parse-pci-interrupt", feature = "to-pci-interrupt"))]
			Self::PciInterrupt(_) => PciInterrupt::CMD_ARG,
			#[cfg(any(feature = "parse-bar-io", feature = "to-bar-io"))]
			Self::BarIo(_) => BarIo::CMD_ARG,
			#[cfg(any(feature = "parse-bar-mmio", feature = "to-bar-mmio"))]
			Self::BarMmio(_) => BarMmio::CMD_ARG,
			Self::Other(o) => str::from_utf8(o).map_err(|_| *o)?,
		})
	}
}

/// Parse arguments from the given iterator
pub fn parse_args<'a, I, F>(mut args: I, mut f: F) -> Result<(), ParseError<'a>>
where
	I: Iterator<Item = &'a [u8]> + 'a,
	F: FnMut(Arg<'a>, &mut I),
{
	while let Some(ty) = args.next() {
		let a = match ty {
			#[cfg(feature = "parse-reg")]
			b"--reg" => Arg::Reg(Reg::from_args(&mut args)?),
			#[cfg(feature = "parse-range")]
			b"--range" => Arg::Range(Range::from_args(&mut args)?),
			#[cfg(feature = "parse-interrupt-map")]
			b"--interrupt-map" => Arg::InterruptMap(InterruptMap::from_args(&mut args)?),
			#[cfg(feature = "parse-interrupt-map-mask")]
			b"--interrupt-map-mask" => Arg::InterruptMapMask(InterruptMapMask::from_args(&mut args)?),
			#[cfg(feature = "parse-pci")]
			b"--pci" => Arg::Pci(Pci::from_args(&mut args)?),
			#[cfg(feature = "parse-pci-interrupt")]
			b"--pci-interrupt" => Arg::PciInterrupt(PciInterrupt::from_args(&mut args)?),
			#[cfg(feature = "parse-bar-mmio")]
			b"--bar-io" => Arg::BarIo(BarIo::from_args(&mut args)?),
			#[cfg(feature = "parse-bar-io")]
			b"--bar-mmio" => Arg::BarMmio(BarMmio::from_args(&mut args)?),
			arg => Arg::Other(arg),
		};
		f(a, &mut args)
	}

	Ok(())
}

/// Convert arguments into static strings.
///
/// Space is allocated using the `alloc` callback. Arguments are added using `add_argument`.
pub fn to_args(args: impl Iterator<Item = ()>) {
	todo!();
}

#[non_exhaustive]
pub enum ParseError<'a> {
	TooManyRegs,
	TooManyRanges,
	MissingArgument(&'static str),
	Utf8Error(str::Utf8Error),
	ParseIntError(num::ParseIntError),
	UnknownArgument(&'a [u8]),
	OutOfMemory,
}

impl<'a> ParseError<'a> {
	#[optimize(size)]
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::TooManyRegs => "too many ranges",
			Self::TooManyRanges => "too many ranges",
			Self::MissingArgument(_) => "expected argument",
			Self::Utf8Error(_) => "utf-8 error",
			Self::ParseIntError(_) => "failed to parse integer",
			Self::UnknownArgument(_) => "unknown argument",
			Self::OutOfMemory => "out of memory",
		}
	}
}

impl fmt::Debug for ParseError<'_> {
	#[optimize(size)]
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::TooManyRegs => fmt::Display::fmt("too many ranges", f),
			Self::TooManyRanges => fmt::Display::fmt("too many ranges", f),
			Self::MissingArgument(r) => write!(f, "expected argument for {:?}", r),
			Self::Utf8Error(r) => r.fmt(f),
			Self::ParseIntError(r) => r.fmt(f),
			Self::UnknownArgument(r) => match str::from_utf8(r) {
				Ok(s) => write!(f, "unknown argument \"--{}\"", s),
				Err(_) => write!(f, "argument is not valid UTF-8"),
			},
			Self::OutOfMemory => fmt::Display::fmt("out of memory", f),
		}
	}
}

impl From<OutOfMemory> for ParseError<'_> {
	#[inline(always)]
	fn from(_: OutOfMemory) -> Self {
		Self::OutOfMemory
	}
}
