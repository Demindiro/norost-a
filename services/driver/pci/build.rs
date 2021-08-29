use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

const BASE_DIR: &str = "../../..";
const LIST: &str = "pci_fs.list";

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
		//name: &'static str,
		vendor: u16,
		device: u16,
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
		let (name, vendor, device, path) = line
			.split_once(char::is_whitespace)
			.map(|(n, r)| (n, r.trim_start()))
			.and_then(|(n, r)| r.split_once(char::is_whitespace).map(|(v, r)| (n, v, r)))
			.map(|(n, v, r)| (n, v, r.trim_start()))
			.and_then(|(n, v, r)| r.split_once(char::is_whitespace).map(|(d, p)| (n, v, d, p)))
			.map(|(n, v, d, p)| (n, v, d, p.trim_start()))
			.expect("expected name, compatibility and path");
		dbg!(name, vendor, device, path);
		let path = if &path[0..1] != "/" {
			format!("{}/{}/{}", base_dir, BASE_DIR, path)
		} else {
			String::from(line)
		};
		write!(
			out,
			"{{
			const LENGTH: usize = include_bytes!({:?}).len();
			const ALIGNED: Aligned<LENGTH> = Aligned(*include_bytes!({:?}));
			Binary {{
				//name: {:?},
				vendor: 0x{},
				device: 0x{},
				data: &ALIGNED.0,
			}}
		}},",
			path, path, name, vendor, device,
		)
		.unwrap();
	}
	write!(out, "];").unwrap();
}
