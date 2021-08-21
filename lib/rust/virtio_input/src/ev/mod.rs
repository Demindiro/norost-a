pub mod kbd;

// Extracted from Linux source (include/uapi/linux/input-event-codes.h)

pub mod ev {
	const SYN: u8 = 0x00;
	const KEY: u8 = 0x01;
	const REL: u8 = 0x02;
	const ABS: u8 = 0x03;
	const MSC: u8 = 0x04;
	const SW: u8 = 0x05;

	const LED: u8 = 0x11;

	const SND: u8 = 0x12;

	const REP: u8 = 0x14;
	const FF: u8 = 0x15;
	const PWD: u8 = 0x16;
	const FF_STATUS: u8 = 0x17;

	const MAX: u8 = 0x1f;
	const CNT: u8 = MAX + 1;
}

pub mod syn {
	const REPORT: u8 = 0;
	const CONFIG: u8 = 1;
	const MT_REPORT: u8 = 2;
	const DROPPED: u8 = 3;
	const MAX: u8 = 0xf;
	const CNT: u8 = MAX + 1;
}
