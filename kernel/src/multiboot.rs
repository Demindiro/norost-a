const MAGIC: u32 = 0xE85250D6;

struct MultiBootHeader {
	magic: u32,
	architecture: u32,
	header_length: u32,
	checksum: u32,
}

struct MultiBootTag {
	typ: u16,
	flags: u16,
	size: u32,
}
