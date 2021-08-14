use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

const BASE_DIR: &str = "../../..";
const LIST: &str = "initfs.list";

fn main() {
	let list = format!("{}/{}", BASE_DIR, LIST);

	println!("cargo:rerun-if-changed={}", list);

	let mut list = File::open(list).unwrap();
	let mut s = String::new();
	list.read_to_string(&mut s).unwrap();
	drop(list);
	let list = s;

	let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("list.rs");
	let mut out = File::create(out).unwrap();

	let base_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

	write!(
		out,
		"
	#[repr(align(4096))]
	pub struct Aligned<const S: usize>([u8; S]);

	pub struct Binary {{
		name: &'static str,
		data: &'static [u8],
	}}

	pub const BINARIES: &[Binary] = &[
	"
	)
	.unwrap();
	for line in list
		.split('\n')
		.map(str::trim)
		.filter(|s| !s.is_empty() && &s[0..1] != "#")
	{
		let (name, path) = line
			.split_once(char::is_whitespace)
			.expect("expected name and path");
		let path = path.trim_start();
		dbg!(name, path);
		let path = if &path[0..1] != "/" {
			format!("{}/{}/{}", base_dir, BASE_DIR, path)
		} else {
			String::from(line)
		};
		write!(
			out,
			"{{
			const LENGTH: usize = include_bytes!(\"{}\").len();
			const ALIGNED: Aligned<LENGTH> = Aligned(*include_bytes!(\"{}\"));
			Binary {{
				name: \"{}\",
				data: &ALIGNED.0,
			}}
		}},",
			path, path, name
		)
		.unwrap();
	}
	write!(out, "];").unwrap();
}
