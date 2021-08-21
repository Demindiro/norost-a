mod key {
	//! Shamelessly copied from Linux source because there is absolutely no chance in hell I'm
	//! typing this over manually.

	pub const RESERVED: u16 = 0;
	pub const ESC: u16 = 1;
	pub const _1: u16 = 2;
	pub const _2: u16 = 3;
	pub const _3: u16 = 4;
	pub const _4: u16 = 5;
	pub const _5: u16 = 6;
	pub const _6: u16 = 7;
	pub const _7: u16 = 8;
	pub const _8: u16 = 9;
	pub const _9: u16 = 10;
	pub const _0: u16 = 11;
	pub const MINUS: u16 = 12;
	pub const EQUAL: u16 = 13;
	pub const BACKSPACE: u16 = 14;
	pub const TAB: u16 = 15;
	pub const Q: u16 = 16;
	pub const W: u16 = 17;
	pub const E: u16 = 18;
	pub const R: u16 = 19;
	pub const T: u16 = 20;
	pub const Y: u16 = 21;
	pub const U: u16 = 22;
	pub const I: u16 = 23;
	pub const O: u16 = 24;
	pub const P: u16 = 25;
	pub const LEFTBRACE: u16 = 26;
	pub const RIGHTBRACE: u16 = 27;
	pub const ENTER: u16 = 28;
	pub const LEFTCTRL: u16 = 29;
	pub const A: u16 = 30;
	pub const S: u16 = 31;
	pub const D: u16 = 32;
	pub const F: u16 = 33;
	pub const G: u16 = 34;
	pub const H: u16 = 35;
	pub const J: u16 = 36;
	pub const K: u16 = 37;
	pub const L: u16 = 38;
	pub const SEMICOLON: u16 = 39;
	pub const APOSTROPHE: u16 = 40;
	pub const GRAVE: u16 = 41;
	pub const LEFTSHIFT: u16 = 42;
	pub const BACKSLASH: u16 = 43;
	pub const Z: u16 = 44;
	pub const X: u16 = 45;
	pub const C: u16 = 46;
	pub const V: u16 = 47;
	pub const B: u16 = 48;
	pub const N: u16 = 49;
	pub const M: u16 = 50;
	pub const COMMA: u16 = 51;
	pub const DOT: u16 = 52;
	pub const SLASH: u16 = 53;
	pub const RIGHTSHIFT: u16 = 54;
	pub const KPASTERISK: u16 = 55;
	pub const LEFTALT: u16 = 56;
	pub const SPACE: u16 = 57;
	pub const CAPSLOCK: u16 = 58;
	pub const F1: u16 = 59;
	pub const F2: u16 = 60;
	pub const F3: u16 = 61;
	pub const F4: u16 = 62;
	pub const F5: u16 = 63;
	pub const F6: u16 = 64;
	pub const F7: u16 = 65;
	pub const F8: u16 = 66;
	pub const F9: u16 = 67;
	pub const F10: u16 = 68;
	pub const NUMLOCK: u16 = 69;
	pub const SCROLLLOCK: u16 = 70;
	pub const KP7: u16 = 71;
	pub const KP8: u16 = 72;
	pub const KP9: u16 = 73;
	pub const KPMINUS: u16 = 74;
	pub const KP4: u16 = 75;
	pub const KP5: u16 = 76;
	pub const KP6: u16 = 77;
	pub const KPPLUS: u16 = 78;
	pub const KP1: u16 = 79;
	pub const KP2: u16 = 80;
	pub const KP3: u16 = 81;
	pub const KP0: u16 = 82;
	pub const KPDOT: u16 = 83;

	pub const ZENKAKUHANKAKU: u16 = 85;
	pub const _102ND: u16 = 86;
	pub const F11: u16 = 87;
	pub const F12: u16 = 88;
	pub const RO: u16 = 89;
	pub const KATAKANA: u16 = 90;
	pub const HIRAGANA: u16 = 91;
	pub const HENKAN: u16 = 92;
	pub const KATAKANAHIRAGANA: u16 = 93;
	pub const MUHENKAN: u16 = 94;
	pub const KPJPCOMMA: u16 = 95;
	pub const KPENTER: u16 = 96;
	pub const RIGHTCTRL: u16 = 97;
	pub const KPSLASH: u16 = 98;
	pub const SYSRQ: u16 = 99;
	pub const RIGHTALT: u16 = 100;
	pub const LINEFEED: u16 = 101;
	pub const HOME: u16 = 102;
	pub const UP: u16 = 103;
	pub const PAGEUP: u16 = 104;
	pub const LEFT: u16 = 105;
	pub const RIGHT: u16 = 106;
	pub const END: u16 = 107;
	pub const DOWN: u16 = 108;
	pub const PAGEDOWN: u16 = 109;
	pub const INSERT: u16 = 110;
	pub const DELETE: u16 = 111;
	pub const MACRO: u16 = 112;
	pub const MUTE: u16 = 113;
	pub const VOLUMEDOWN: u16 = 114;
	pub const VOLUMEUP: u16 = 115;
	pub const POWER: u16 = 116;
	pub const KPEQUAL: u16 = 117;
	pub const KPPLUSMINUS: u16 = 118;
	pub const PAUSE: u16 = 119;
	pub const SCALE: u16 = 120;

	pub const KPCOMMA: u16 = 121;
	pub const HANGEUL: u16 = 122;
	pub const HANGUEL: u16 = HANGEUL;
	pub const HANJA: u16 = 123;
	pub const YEN: u16 = 124;
	pub const LEFTMETA: u16 = 125;
	pub const RIGHTMETA: u16 = 126;
	pub const COMPOSE: u16 = 127;

	pub const STOP: u16 = 128;
	pub const AGAIN: u16 = 129;
	pub const PROPS: u16 = 130;
	pub const UNDO: u16 = 131;
	pub const FRONT: u16 = 132;
	pub const COPY: u16 = 133;
	pub const OPEN: u16 = 134;
	pub const PASTE: u16 = 135;
	pub const FIND: u16 = 136;
	pub const CUT: u16 = 137;
	pub const HELP: u16 = 138;
	pub const MENU: u16 = 139;
	pub const CALC: u16 = 140;
	pub const SETUP: u16 = 141;
	pub const SLEEP: u16 = 142;
	pub const WAKEUP: u16 = 143;
	pub const FILE: u16 = 144;
	pub const SENDFILE: u16 = 145;
	pub const DELETEFILE: u16 = 146;
	pub const XFER: u16 = 147;
	pub const PROG1: u16 = 148;
	pub const PROG2: u16 = 149;
	pub const WWW: u16 = 150;
	pub const MSDOS: u16 = 151;
	pub const COFFEE: u16 = 152;
	pub const SCREENLOCK: u16 = COFFEE;
	pub const ROTATE_DISPLAY: u16 = 153;
	pub const DIRECTION: u16 = ROTATE_DISPLAY;
	pub const CYCLEWINDOWS: u16 = 154;
	pub const MAIL: u16 = 155;
	pub const BOOKMARKS: u16 = 156;
	pub const COMPUTER: u16 = 157;
	pub const BACK: u16 = 158;
	pub const FORWARD: u16 = 159;
	pub const CLOSECD: u16 = 160;
	pub const EJECTCD: u16 = 161;
	pub const EJECTCLOSECD: u16 = 162;
	pub const NEXTSONG: u16 = 163;
	pub const PLAYPAUSE: u16 = 164;
	pub const PREVIOUSSONG: u16 = 165;
	pub const STOPCD: u16 = 166;
	pub const RECORD: u16 = 167;
	pub const REWIND: u16 = 168;
	pub const PHONE: u16 = 169;
	pub const ISO: u16 = 170;
	pub const CONFIG: u16 = 171;
	pub const HOMEPAGE: u16 = 172;
	pub const REFRESH: u16 = 173;
	pub const EXIT: u16 = 174;
	pub const MOVE: u16 = 175;
	pub const EDIT: u16 = 176;
	pub const SCROLLUP: u16 = 177;
	pub const SCROLLDOWN: u16 = 178;
	pub const KPLEFTPAREN: u16 = 179;
	pub const KPRIGHTPAREN: u16 = 180;
	pub const NEW: u16 = 181;
	pub const REDO: u16 = 182;

	pub const F13: u16 = 183;
	pub const F14: u16 = 184;
	pub const F15: u16 = 185;
	pub const F16: u16 = 186;
	pub const F17: u16 = 187;
	pub const F18: u16 = 188;
	pub const F19: u16 = 189;
	pub const F20: u16 = 190;
	pub const F21: u16 = 191;
	pub const F22: u16 = 192;
	pub const F23: u16 = 193;
	pub const F24: u16 = 194;

	pub const PLAYCD: u16 = 200;
	pub const PAUSECD: u16 = 201;
	pub const PROG3: u16 = 202;
	pub const PROG4: u16 = 203;
	pub const DASHBOARD: u16 = 204;
	pub const SUSPEND: u16 = 205;
	pub const CLOSE: u16 = 206;
	pub const PLAY: u16 = 207;
	pub const FASTFORWARD: u16 = 208;
	pub const BASSBOOST: u16 = 209;
	pub const PRINT: u16 = 210;
	pub const HP: u16 = 211;
	pub const CAMERA: u16 = 212;
	pub const SOUND: u16 = 213;
	pub const QUESTION: u16 = 214;
	pub const EMAIL: u16 = 215;
	pub const CHAT: u16 = 216;
	pub const SEARCH: u16 = 217;
	pub const CONNECT: u16 = 218;
	pub const FINANCE: u16 = 219;
	pub const SPORT: u16 = 220;
	pub const SHOP: u16 = 221;
	pub const ALTERASE: u16 = 222;
	pub const CANCEL: u16 = 223;
	pub const BRIGHTNESSDOWN: u16 = 224;
	pub const BRIGHTNESSUP: u16 = 225;
	pub const MEDIA: u16 = 226;

	pub const SWITCHVIDEOMODE: u16 = 227;
	pub const KBDILLUMTOGGLE: u16 = 228;
	pub const KBDILLUMDOWN: u16 = 229;
	pub const KBDILLUMUP: u16 = 230;

	pub const SEND: u16 = 231;
	pub const REPLY: u16 = 232;
	pub const FORWARDMAIL: u16 = 233;
	pub const SAVE: u16 = 234;
	pub const DOCUMENTS: u16 = 235;

	pub const BATTERY: u16 = 236;

	pub const BLUETOOTH: u16 = 237;
	pub const WLAN: u16 = 238;
	pub const UWB: u16 = 239;

	pub const UNKNOWN: u16 = 240;

	pub const VIDEO_NEXT: u16 = 241;
	pub const VIDEO_PREV: u16 = 242;
	pub const BRIGHTNESS_CYCLE: u16 = 243;
	pub const BRIGHTNESS_AUTO: u16 = 244;
	pub const BRIGHTNESS_ZERO: u16 = BRIGHTNESS_AUTO;
	pub const DISPLAY_OFF: u16 = 245;

	pub const WWAN: u16 = 246;
	pub const WIMAX: u16 = WWAN;
	pub const RFKILL: u16 = 247;

	pub const MICMUTE: u16 = 248;

	pub const OK: u16 = 0x160;
	pub const SELECT: u16 = 0x161;
	pub const GOTO: u16 = 0x162;
	pub const CLEAR: u16 = 0x163;
	pub const POWER2: u16 = 0x164;
	pub const OPTION: u16 = 0x165;
	pub const INFO: u16 = 0x166;
	pub const TIME: u16 = 0x167;
	pub const VENDOR: u16 = 0x168;
	pub const ARCHIVE: u16 = 0x169;
	pub const PROGRAM: u16 = 0x16a;
	pub const CHANNEL: u16 = 0x16b;
	pub const FAVORITES: u16 = 0x16c;
	pub const EPG: u16 = 0x16d;
	pub const PVR: u16 = 0x16e;
	pub const MHP: u16 = 0x16f;
	pub const LANGUAGE: u16 = 0x170;
	pub const TITLE: u16 = 0x171;
	pub const SUBTITLE: u16 = 0x172;
	pub const ANGLE: u16 = 0x173;
	pub const FULL_SCREEN: u16 = 0x174;
	pub const ZOOM: u16 = FULL_SCREEN;
	pub const MODE: u16 = 0x175;
	pub const KEYBOARD: u16 = 0x176;
	pub const ASPECT_RATIO: u16 = 0x177;
	pub const SCREEN: u16 = ASPECT_RATIO;
	pub const PC: u16 = 0x178;
	pub const TV: u16 = 0x179;
	pub const TV2: u16 = 0x17a;
	pub const VCR: u16 = 0x17b;
	pub const VCR2: u16 = 0x17c;
	pub const SAT: u16 = 0x17d;
	pub const SAT2: u16 = 0x17e;
	pub const CD: u16 = 0x17f;
	pub const TAPE: u16 = 0x180;
	pub const RADIO: u16 = 0x181;
	pub const TUNER: u16 = 0x182;
	pub const PLAYER: u16 = 0x183;
	pub const TEXT: u16 = 0x184;
	pub const DVD: u16 = 0x185;
	pub const AUX: u16 = 0x186;
	pub const MP3: u16 = 0x187;
	pub const AUDIO: u16 = 0x188;
	pub const VIDEO: u16 = 0x189;
	pub const DIRECTORY: u16 = 0x18a;
	pub const LIST: u16 = 0x18b;
	pub const MEMO: u16 = 0x18c;
	pub const CALENDAR: u16 = 0x18d;
	pub const RED: u16 = 0x18e;
	pub const GREEN: u16 = 0x18f;
	pub const YELLOW: u16 = 0x190;
	pub const BLUE: u16 = 0x191;
	pub const CHANNELUP: u16 = 0x192;
	pub const CHANNELDOWN: u16 = 0x193;
	pub const FIRST: u16 = 0x194;
	pub const LAST: u16 = 0x195;
	pub const AB: u16 = 0x196;
	pub const NEXT: u16 = 0x197;
	pub const RESTART: u16 = 0x198;
	pub const SLOW: u16 = 0x199;
	pub const SHUFFLE: u16 = 0x19a;
	pub const BREAK: u16 = 0x19b;
	pub const PREVIOUS: u16 = 0x19c;
	pub const DIGITS: u16 = 0x19d;
	pub const TEEN: u16 = 0x19e;
	pub const TWEN: u16 = 0x19f;
	pub const VIDEOPHONE: u16 = 0x1a0;
	pub const GAMES: u16 = 0x1a1;
	pub const ZOOMIN: u16 = 0x1a2;
	pub const ZOOMOUT: u16 = 0x1a3;
	pub const ZOOMRESET: u16 = 0x1a4;
	pub const WORDPROCESSOR: u16 = 0x1a5;
	pub const EDITOR: u16 = 0x1a6;
	pub const SPREADSHEET: u16 = 0x1a7;
	pub const GRAPHICSEDITOR: u16 = 0x1a8;
	pub const PRESENTATION: u16 = 0x1a9;
	pub const DATABASE: u16 = 0x1aa;
	pub const NEWS: u16 = 0x1ab;
	pub const VOICEMAIL: u16 = 0x1ac;
	pub const ADDRESSBOOK: u16 = 0x1ad;
	pub const MESSENGER: u16 = 0x1ae;
	pub const DISPLAYTOGGLE: u16 = 0x1af;
	pub const BRIGHTNESS_TOGGLE: u16 = DISPLAYTOGGLE;
	pub const SPELLCHECK: u16 = 0x1b0;
	pub const LOGOFF: u16 = 0x1b1;

	pub const DOLLAR: u16 = 0x1b2;
	pub const EURO: u16 = 0x1b3;

	pub const FRAMEBACK: u16 = 0x1b4;
	pub const FRAMEFORWARD: u16 = 0x1b5;
	pub const CONTEXT_MENU: u16 = 0x1b6;
	pub const MEDIA_REPEAT: u16 = 0x1b7;
	pub const _10CHANNELSUP: u16 = 0x1b8;
	pub const _10CHANNELSDOWN: u16 = 0x1b9;
	pub const IMAGES: u16 = 0x1ba;
	pub const NOTIFICATION_CENTER: u16 = 0x1bc;
	pub const PICKUP_PHONE: u16 = 0x1bd;
	pub const HANGUP_PHONE: u16 = 0x1be;

	pub const DEL_EOL: u16 = 0x1c0;
	pub const DEL_EOS: u16 = 0x1c1;
	pub const INS_LINE: u16 = 0x1c2;
	pub const DEL_LINE: u16 = 0x1c3;

	pub const FN: u16 = 0x1d0;
	pub const FN_ESC: u16 = 0x1d1;
	pub const FN_F1: u16 = 0x1d2;
	pub const FN_F2: u16 = 0x1d3;
	pub const FN_F3: u16 = 0x1d4;
	pub const FN_F4: u16 = 0x1d5;
	pub const FN_F5: u16 = 0x1d6;
	pub const FN_F6: u16 = 0x1d7;
	pub const FN_F7: u16 = 0x1d8;
	pub const FN_F8: u16 = 0x1d9;
	pub const FN_F9: u16 = 0x1da;
	pub const FN_F10: u16 = 0x1db;
	pub const FN_F11: u16 = 0x1dc;
	pub const FN_F12: u16 = 0x1dd;
	pub const FN_1: u16 = 0x1de;
	pub const FN_2: u16 = 0x1df;
	pub const FN_D: u16 = 0x1e0;
	pub const FN_E: u16 = 0x1e1;
	pub const FN_F: u16 = 0x1e2;
	pub const FN_S: u16 = 0x1e3;
	pub const FN_B: u16 = 0x1e4;
	pub const FN_RIGHT_SHIFT: u16 = 0x1e5;

	pub const BRL_DOT1: u16 = 0x1f1;
	pub const BRL_DOT2: u16 = 0x1f2;
	pub const BRL_DOT3: u16 = 0x1f3;
	pub const BRL_DOT4: u16 = 0x1f4;
	pub const BRL_DOT5: u16 = 0x1f5;
	pub const BRL_DOT6: u16 = 0x1f6;
	pub const BRL_DOT7: u16 = 0x1f7;
	pub const BRL_DOT8: u16 = 0x1f8;
	pub const BRL_DOT9: u16 = 0x1f9;
	pub const BRL_DOT10: u16 = 0x1fa;

	pub const NUMERIC_0: u16 = 0x200;
	pub const NUMERIC_1: u16 = 0x201;
	pub const NUMERIC_2: u16 = 0x202;
	pub const NUMERIC_3: u16 = 0x203;
	pub const NUMERIC_4: u16 = 0x204;
	pub const NUMERIC_5: u16 = 0x205;
	pub const NUMERIC_6: u16 = 0x206;
	pub const NUMERIC_7: u16 = 0x207;
	pub const NUMERIC_8: u16 = 0x208;
	pub const NUMERIC_9: u16 = 0x209;
	pub const NUMERIC_STAR: u16 = 0x20a;
	pub const NUMERIC_POUND: u16 = 0x20b;
	pub const NUMERIC_A: u16 = 0x20c;
	pub const NUMERIC_B: u16 = 0x20d;
	pub const NUMERIC_C: u16 = 0x20e;
	pub const NUMERIC_D: u16 = 0x20f;

	pub const CAMERA_FOCUS: u16 = 0x210;
	pub const WPS_BUTTON: u16 = 0x211;

	pub const TOUCHPAD_TOGGLE: u16 = 0x212;
	pub const TOUCHPAD_ON: u16 = 0x213;
	pub const TOUCHPAD_OFF: u16 = 0x214;

	pub const CAMERA_ZOOMIN: u16 = 0x215;
	pub const CAMERA_ZOOMOUT: u16 = 0x216;
	pub const CAMERA_UP: u16 = 0x217;
	pub const CAMERA_DOWN: u16 = 0x218;
	pub const CAMERA_LEFT: u16 = 0x219;
	pub const CAMERA_RIGHT: u16 = 0x21a;

	pub const ATTENDANT_ON: u16 = 0x21b;
	pub const ATTENDANT_OFF: u16 = 0x21c;
	pub const ATTENDANT_TOGGLE: u16 = 0x21d;
	pub const LIGHTS_TOGGLE: u16 = 0x21e;

	pub const ALS_TOGGLE: u16 = 0x230;
	pub const ROTATE_LOCK_TOGGLE: u16 = 0x231;

	pub const BUTTONCONFIG: u16 = 0x240;
	pub const TASKMANAGER: u16 = 0x241;
	pub const JOURNAL: u16 = 0x242;
	pub const CONTROLPANEL: u16 = 0x243;
	pub const APPSELECT: u16 = 0x244;
	pub const SCREENSAVER: u16 = 0x245;
	pub const VOICECOMMAND: u16 = 0x246;
	pub const ASSISTANT: u16 = 0x247;
	pub const KBD_LAYOUT_NEXT: u16 = 0x248;

	pub const BRIGHTNESS_MIN: u16 = 0x250;
	pub const BRIGHTNESS_MAX: u16 = 0x251;

	pub const KBDINPUTASSIST_PREV: u16 = 0x260;
	pub const KBDINPUTASSIST_NEXT: u16 = 0x261;
	pub const KBDINPUTASSIST_PREVGROUP: u16 = 0x262;
	pub const KBDINPUTASSIST_NEXTGROUP: u16 = 0x263;
	pub const KBDINPUTASSIST_ACCEPT: u16 = 0x264;
	pub const KBDINPUTASSIST_CANCEL: u16 = 0x265;

	pub const RIGHT_UP: u16 = 0x266;
	pub const RIGHT_DOWN: u16 = 0x267;
	pub const LEFT_UP: u16 = 0x268;
	pub const LEFT_DOWN: u16 = 0x269;

	pub const ROOT_MENU: u16 = 0x26a;

	pub const MEDIA_TOP_MENU: u16 = 0x26b;
	pub const NUMERIC_11: u16 = 0x26c;
	pub const NUMERIC_12: u16 = 0x26d;
	/*
	 * Toggle Audio Description: refers to an audio service that helps blind and
	 * visually impaired consumers understand the action in a program. Note: in
	 * some countries this is referred to as "Video Description".
	 */
	pub const AUDIO_DESC: u16 = 0x26e;
	pub const _3D_MODE: u16 = 0x26f;
	pub const NEXT_FAVORITE: u16 = 0x270;
	pub const STOP_RECORD: u16 = 0x271;
	pub const PAUSE_RECORD: u16 = 0x272;
	pub const VOD: u16 = 0x273;
	pub const UNMUTE: u16 = 0x274;
	pub const FASTREVERSE: u16 = 0x275;
	pub const SLOWREVERSE: u16 = 0x276;
	/*
	 * Control a data application associated with the currently viewed channel,
	 * e.g. teletext or data broadcast application (MHEG, MHP, HbbTV, etc.)
	 */
	pub const DATA: u16 = 0x277;
	pub const ONSCREEN_KEYBOARD: u16 = 0x278;

	pub const PRIVACY_SCREEN_TOGGLE: u16 = 0x279;

	pub const SELECTIVE_SCREENSHOT: u16 = 0x27a;

	/*
	 * Some keyboards have keys which do not have a defined meaning, these keys
	 * are intended to be programmed / bound to macros by the user. For most
	 * keyboards with these macro-keys the key-sequence to inject, or action to
	 * take, is all handled by software on the host side. So from the kernel's
	 * point of view these are just normal keys.
	 *
	 * The MACRO# codes below are intended for such keys, which may be labeled
	 * e.g. G1-G18, or S1 - S30. The MACRO# codes MUST NOT be used for keys
	 * where the marking on the key does indicate a defined meaning / purpose.
	 *
	 * The MACRO# codes MUST also NOT be used as fallback for when no existing
	 * FOO define matches the marking / purpose. In this case a new KEY_FOO
	 * define MUST be added.
	 */
	pub const MACRO1: u16 = 0x290;
	pub const MACRO2: u16 = 0x291;
	pub const MACRO3: u16 = 0x292;
	pub const MACRO4: u16 = 0x293;
	pub const MACRO5: u16 = 0x294;
	pub const MACRO6: u16 = 0x295;
	pub const MACRO7: u16 = 0x296;
	pub const MACRO8: u16 = 0x297;
	pub const MACRO9: u16 = 0x298;
	pub const MACRO10: u16 = 0x299;
	pub const MACRO11: u16 = 0x29a;
	pub const MACRO12: u16 = 0x29b;
	pub const MACRO13: u16 = 0x29c;
	pub const MACRO14: u16 = 0x29d;
	pub const MACRO15: u16 = 0x29e;
	pub const MACRO16: u16 = 0x29f;
	pub const MACRO17: u16 = 0x2a0;
	pub const MACRO18: u16 = 0x2a1;
	pub const MACRO19: u16 = 0x2a2;
	pub const MACRO20: u16 = 0x2a3;
	pub const MACRO21: u16 = 0x2a4;
	pub const MACRO22: u16 = 0x2a5;
	pub const MACRO23: u16 = 0x2a6;
	pub const MACRO24: u16 = 0x2a7;
	pub const MACRO25: u16 = 0x2a8;
	pub const MACRO26: u16 = 0x2a9;
	pub const MACRO27: u16 = 0x2aa;
	pub const MACRO28: u16 = 0x2ab;
	pub const MACRO29: u16 = 0x2ac;
	pub const MACRO30: u16 = 0x2ad;

	pub const MACRO_RECORD_START: u16 = 0x2b0;
	pub const MACRO_RECORD_STOP: u16 = 0x2b1;
	pub const MACRO_PRESET_CYCLE: u16 = 0x2b2;
	pub const MACRO_PRESET1: u16 = 0x2b3;
	pub const MACRO_PRESET2: u16 = 0x2b4;
	pub const MACRO_PRESET3: u16 = 0x2b5;

	pub const KBD_LCD_MENU1: u16 = 0x2b8;
	pub const KBD_LCD_MENU2: u16 = 0x2b9;
	pub const KBD_LCD_MENU3: u16 = 0x2ba;
	pub const KBD_LCD_MENU4: u16 = 0x2bb;
	pub const KBD_LCD_MENU5: u16 = 0x2bc;

	pub const MIN_INTERESTING: u16 = MUTE;
	pub const MAX: u16 = 0x2ff;
	pub const CNT: u16 = MAX + 1;
}

pub mod btn {
	pub const MISC: u16 = 0x100;
	pub const _0: u16 = 0x100;
	pub const _1: u16 = 0x101;
	pub const _2: u16 = 0x102;
	pub const _3: u16 = 0x103;
	pub const _4: u16 = 0x104;
	pub const _5: u16 = 0x105;
	pub const _6: u16 = 0x106;
	pub const _7: u16 = 0x107;
	pub const _8: u16 = 0x108;
	pub const _9: u16 = 0x109;

	pub const MOUSE: u16 = 0x110;
	pub const LEFT: u16 = 0x110;
	pub const RIGHT: u16 = 0x111;
	pub const MIDDLE: u16 = 0x112;
	pub const SIDE: u16 = 0x113;
	pub const EXTRA: u16 = 0x114;
	pub const FORWARD: u16 = 0x115;
	pub const BACK: u16 = 0x116;
	pub const TASK: u16 = 0x117;

	pub const JOYSTICK: u16 = 0x120;
	pub const TRIGGER: u16 = 0x120;
	pub const THUMB: u16 = 0x121;
	pub const THUMB2: u16 = 0x122;
	pub const TOP: u16 = 0x123;
	pub const TOP2: u16 = 0x124;
	pub const PINKIE: u16 = 0x125;
	pub const BASE: u16 = 0x126;
	pub const BASE2: u16 = 0x127;
	pub const BASE3: u16 = 0x128;
	pub const BASE4: u16 = 0x129;
	pub const BASE5: u16 = 0x12a;
	pub const BASE6: u16 = 0x12b;
	pub const DEAD: u16 = 0x12f;

	pub const GAMEPAD: u16 = 0x130;
	pub const SOUTH: u16 = 0x130;
	pub const A: u16 = SOUTH;
	pub const EAST: u16 = 0x131;
	pub const B: u16 = EAST;
	pub const C: u16 = 0x132;
	pub const NORTH: u16 = 0x133;
	pub const X: u16 = NORTH;
	pub const WEST: u16 = 0x134;
	pub const Y: u16 = WEST;
	pub const Z: u16 = 0x135;
	pub const TL: u16 = 0x136;
	pub const TR: u16 = 0x137;
	pub const TL2: u16 = 0x138;
	pub const TR2: u16 = 0x139;
	pub const SELECT: u16 = 0x13a;
	pub const START: u16 = 0x13b;
	pub const MODE: u16 = 0x13c;
	pub const THUMBL: u16 = 0x13d;
	pub const THUMBR: u16 = 0x13e;

	pub const DIGI: u16 = 0x140;
	pub const TOOL_PEN: u16 = 0x140;
	pub const TOOL_RUBBER: u16 = 0x141;
	pub const TOOL_BRUSH: u16 = 0x142;
	pub const TOOL_PENCIL: u16 = 0x143;
	pub const TOOL_AIRBRUSH: u16 = 0x144;
	pub const TOOL_FINGER: u16 = 0x145;
	pub const TOOL_MOUSE: u16 = 0x146;
	pub const TOOL_LENS: u16 = 0x147;
	pub const TOOL_QUINTTAP: u16 = 0x148;
	pub const STYLUS3: u16 = 0x149;
	pub const TOUCH: u16 = 0x14a;
	pub const STYLUS: u16 = 0x14b;
	pub const STYLUS2: u16 = 0x14c;
	pub const TOOL_DOUBLETAP: u16 = 0x14d;
	pub const TOOL_TRIPLETAP: u16 = 0x14e;
	pub const TOOL_QUADTAP: u16 = 0x14f;

	pub const WHEEL: u16 = 0x150;
	pub const GEAR_DOWN: u16 = 0x150;
	pub const GEAR_UP: u16 = 0x151;

	pub const DPAD_UP: u16 = 0x220;
	pub const DPAD_DOWN: u16 = 0x221;
	pub const DPAD_LEFT: u16 = 0x222;
	pub const DPAD_RIGHT: u16 = 0x223;

	pub const TRIGGER_HAPPY: u16 = 0x2c0;
	pub const TRIGGER_HAPPY1: u16 = 0x2c0;
	pub const TRIGGER_HAPPY2: u16 = 0x2c1;
	pub const TRIGGER_HAPPY3: u16 = 0x2c2;
	pub const TRIGGER_HAPPY4: u16 = 0x2c3;
	pub const TRIGGER_HAPPY5: u16 = 0x2c4;
	pub const TRIGGER_HAPPY6: u16 = 0x2c5;
	pub const TRIGGER_HAPPY7: u16 = 0x2c6;
	pub const TRIGGER_HAPPY8: u16 = 0x2c7;
	pub const TRIGGER_HAPPY9: u16 = 0x2c8;
	pub const TRIGGER_HAPPY10: u16 = 0x2c9;
	pub const TRIGGER_HAPPY11: u16 = 0x2ca;
	pub const TRIGGER_HAPPY12: u16 = 0x2cb;
	pub const TRIGGER_HAPPY13: u16 = 0x2cc;
	pub const TRIGGER_HAPPY14: u16 = 0x2cd;
	pub const TRIGGER_HAPPY15: u16 = 0x2ce;
	pub const TRIGGER_HAPPY16: u16 = 0x2cf;
	pub const TRIGGER_HAPPY17: u16 = 0x2d0;
	pub const TRIGGER_HAPPY18: u16 = 0x2d1;
	pub const TRIGGER_HAPPY19: u16 = 0x2d2;
	pub const TRIGGER_HAPPY20: u16 = 0x2d3;
	pub const TRIGGER_HAPPY21: u16 = 0x2d4;
	pub const TRIGGER_HAPPY22: u16 = 0x2d5;
	pub const TRIGGER_HAPPY23: u16 = 0x2d6;
	pub const TRIGGER_HAPPY24: u16 = 0x2d7;
	pub const TRIGGER_HAPPY25: u16 = 0x2d8;
	pub const TRIGGER_HAPPY26: u16 = 0x2d9;
	pub const TRIGGER_HAPPY27: u16 = 0x2da;
	pub const TRIGGER_HAPPY28: u16 = 0x2db;
	pub const TRIGGER_HAPPY29: u16 = 0x2dc;
	pub const TRIGGER_HAPPY30: u16 = 0x2dd;
	pub const TRIGGER_HAPPY31: u16 = 0x2de;
	pub const TRIGGER_HAPPY32: u16 = 0x2df;
	pub const TRIGGER_HAPPY33: u16 = 0x2e0;
	pub const TRIGGER_HAPPY34: u16 = 0x2e1;
	pub const TRIGGER_HAPPY35: u16 = 0x2e2;
	pub const TRIGGER_HAPPY36: u16 = 0x2e3;
	pub const TRIGGER_HAPPY37: u16 = 0x2e4;
	pub const TRIGGER_HAPPY38: u16 = 0x2e5;
	pub const TRIGGER_HAPPY39: u16 = 0x2e6;
	pub const TRIGGER_HAPPY40: u16 = 0x2e7;
}
