//! # Helper library for drivers

#![no_std]

use core::fmt;
use core::num;
use core::str;

macro_rules! derive {
	($name:ident $a:ident $b:ident) => {
		#[derive(Clone, Copy)]
		pub struct $name {
			pub $a: u128,
			pub $b: u128,
		}

		impl fmt::Debug for $name {
			fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
				fmt2(
					f,
					stringify!($name),
					stringify!($a),
					stringify!($b),
					self.$a,
					self.$b,
				)
			}
		}
	};
	($name:ident $a:ident $b:ident $c:ident) => {
		#[derive(Clone, Copy)]
		pub struct $name {
			pub $a: u128,
			pub $b: u128,
			pub $c: u128,
		}

		impl fmt::Debug for $name {
			fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
				fmt3(
					f,
					stringify!($name),
					stringify!($a),
					stringify!($b),
					stringify!($c),
					self.$a,
					self.$b,
					self.$c,
				)
			}
		}
	};
}

fn fmt2(f: &mut fmt::Formatter, name: &str, na: &str, nb: &str, a: u128, b: u128) -> fmt::Result {
	f.debug_struct(name)
		.field(na, &format_args!("0x{:x}", a))
		.field(nb, &format_args!("0x{:x}", b))
		.finish()
}

fn fmt3(
	f: &mut fmt::Formatter,
	name: &str,
	na: &str,
	nb: &str,
	nc: &str,
	a: u128,
	b: u128,
	c: u128,
) -> fmt::Result {
	f.debug_struct(name)
		.field(na, &format_args!("0x{:x}", a))
		.field(nb, &format_args!("0x{:x}", b))
		.field(nc, &format_args!("0x{:x}", c))
		.finish()
}

fn parse2<'a, I>(args: &mut I, name: &'static str) -> Result<(u128, u128), ParseError<'a>>
where
	I: Iterator<Item = &'a [u8]> + 'a,
{
	let a = args.next().ok_or(ParseError::MissingArgument(name))?;
	let b = args.next().ok_or(ParseError::MissingArgument(name))?;
	let a = str::from_utf8(a).map_err(ParseError::Utf8Error)?;
	let b = str::from_utf8(b).map_err(ParseError::Utf8Error)?;
	let a = u128::from_str_radix(a, 16).map_err(ParseError::ParseIntError)?;
	let b = u128::from_str_radix(b, 16).map_err(ParseError::ParseIntError)?;
	Ok((a, b))
}

fn parse3<'a, I>(args: &mut I, name: &'static str) -> Result<(u128, u128, u128), ParseError<'a>>
where
	I: Iterator<Item = &'a [u8]> + 'a,
{
	let a = args.next().ok_or(ParseError::MissingArgument(name))?;
	let b = args.next().ok_or(ParseError::MissingArgument(name))?;
	let c = args.next().ok_or(ParseError::MissingArgument(name))?;
	let a = str::from_utf8(a).map_err(ParseError::Utf8Error)?;
	let b = str::from_utf8(b).map_err(ParseError::Utf8Error)?;
	let c = str::from_utf8(c).map_err(ParseError::Utf8Error)?;
	let a = u128::from_str_radix(a, 16).map_err(ParseError::ParseIntError)?;
	let b = u128::from_str_radix(b, 16).map_err(ParseError::ParseIntError)?;
	let c = u128::from_str_radix(c, 16).map_err(ParseError::ParseIntError)?;
	Ok((a, b, c))
}

derive!(Reg address size);
derive!(Range child_address address size);
derive!(Pci address size);
derive!(BarMmio index address size);
derive!(BarIo index address size);

#[derive(Debug)]
#[non_exhaustive]
pub enum Arg<'a> {
	Reg(Reg),
	Range(Range),
	Pci(Pci),
	BarIo(BarIo),
	BarMmio(BarMmio),
	Other(&'a [u8]),
}

/// Parse arguments from the given iterator
pub fn parse_args<'a, I, F>(mut args: I, mut f: F) -> Result<(), ParseError<'a>>
where
	I: Iterator<Item = &'a [u8]> + 'a,
	F: FnMut(Arg<'a>, &mut I),
{
	while let Some(ty) = args.next() {
		match ty {
			b"--reg" => {
				let (address, size) = parse2(&mut args, "--reg")?;
				f(Arg::Reg(Reg { address, size }), &mut args);
			}
			b"--range" => {
				let (child_address, address, size) = parse3(&mut args, "--range")?;
				f(
					Arg::Range(Range {
						child_address,
						address,
						size,
					}),
					&mut args,
				);
			}
			b"--pci" => {
				let (address, size) = parse2(&mut args, "--pci")?;
				f(Arg::Pci(Pci { address, size }), &mut args);
			}
			b"--bar-io" => {
				let (index, address, size) = parse3(&mut args, "--bar-io")?;
				f(
					Arg::BarIo(BarIo {
						index,
						address,
						size,
					}),
					&mut args,
				);
			}
			b"--bar-mmio" => {
				let (index, address, size) = parse3(&mut args, "--bar-mmio")?;
				f(
					Arg::BarMmio(BarMmio {
						index,
						address,
						size,
					}),
					&mut args,
				);
			}
			arg => f(Arg::Other(arg), &mut args),
		}
	}

	Ok(())
}

#[non_exhaustive]
pub enum ParseError<'a> {
	TooManyRegs,
	TooManyRanges,
	MissingArgument(&'static str),
	Utf8Error(str::Utf8Error),
	ParseIntError(num::ParseIntError),
	UnknownArgument(&'a [u8]),
}

impl fmt::Debug for ParseError<'_> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::TooManyRegs => fmt::Display::fmt("too many ranges", f),
			Self::TooManyRanges => fmt::Display::fmt("too many ranges", f),
			Self::MissingArgument(r) => write!(f, "expected argument for {:?}", r),
			Self::Utf8Error(r) => r.fmt(f),
			Self::ParseIntError(r) => r.fmt(f),
			Self::UnknownArgument(r) => write!(f, "unknown argument {:?}", str::from_utf8(r)),
		}
	}
}
