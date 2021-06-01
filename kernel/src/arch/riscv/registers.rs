/// Structure used to save register state
#[repr(C)]
pub struct RegisterState {
	/// All integer registers except `x0`
	x: [usize; 31],
	/// The program counter state.
	pc: usize,
	// /// All FP registers
	//f: [usize; 32],
}

/// Save/load a single register. Intended for internal use.
#[cfg(target_pointer_width = "64")]
macro_rules! s {
	($reg:ident -> $state:ident[$offset:literal]) => {
		asm!(concat!("sd	", stringify!($reg), ", ", $offset, "*8(", stringify!($state), ")");
	};
	($reg:ident <- $state:ident[$offset:literal]) => {
		asm!(concat!("ld	", stringify!($reg), ", ", $offset, "*8(", stringify!($state), ")");
	};
}
#[cfg(target_pointer_width = "32")]
macro_rules! s {
	($reg:ident -> $state:ident[$offset:literal]) => {
		asm!(concat!("sd	", stringify!($reg), ", ", $offset, "*4(", stringify!($state), ")");
	};
	($reg:ident <- $state:ident[$offset:literal]) => {
		asm!(concat!("ld	", stringify!($reg), ", ", $offset, "*4(", stringify!($state), ")");
	};
}

/// Save/load only the caller-saved integer registers. Normally used in the interrupt
/// pro-/epilogue.
///
/// The first argument indicates which register the pointer to the `RegisterState` is located in.
pub macro_rules! caller {
	($state:literal) => {
		s!(ra -> $state[0]);
		s!(t0 -> $state[1]);
		s!(t1 -> $state[2]);
		s!(t2 -> $state[3]);
		s!(t3 -> $state[4]);
		s!(t4 -> $state[5]);
		s!(t5 -> $state[6]);
		s!(t6 -> $state[7]);
		s!(a0 -> $state[8]);
		s!(a1 -> $state[9]);
		s!(a2 -> $state[10]);
		s!(a3 -> $state[11]);
		s!(a4 -> $state[12]);
		s!(a5 -> $state[13]);
		s!(a6 -> $state[14]);
		s!(a7 -> $state[15]);
	};
	(load $reg:literal) => {
		s!(ra <- $state[0]);
		s!(t0 <- $state[1]);
		s!(t1 <- $state[2]);
		s!(t2 <- $state[3]);
		s!(t3 <- $state[4]);
		s!(t4 <- $state[5]);
		s!(t5 <- $state[6]);
		s!(t6 <- $state[7]);
		s!(a0 <- $state[8]);
		s!(a1 <- $state[9]);
		s!(a2 <- $state[10]);
		s!(a3 <- $state[11]);
		s!(a4 <- $state[12]);
		s!(a5 <- $state[13]);
		s!(a6 <- $state[14]);
		s!(a7 <- $state[15]);
	};
}

/// Save/load only the callee-saved integer registers _except_ `ra`. Normally used for context
/// switching.
pub macro_rules! caller {
	(save $state:ident) => {
		s!(sp -> $state[16]);
		s!(gp -> $state[17]);
		s!(tp -> $state[18]);
		s!(s0 -> $state[19]);
		s!(s1 -> $state[20]);
		s!(s2 -> $state[21]);
		s!(s3 -> $state[22]);
		s!(s4 -> $state[23]);
		s!(s5 -> $state[24]);
		s!(s6 -> $state[25]);
		s!(s7 -> $state[26]);
		s!(s8 -> $state[27]);
		s!(s9 -> $state[28]);
		s!(s10 -> $state[29]);
		s!(s11 -> $state[30]);
	};
	(load $reg:literal) => {
		s!(sp <- $state[16]);
		s!(gp <- $state[17]);
		s!(tp <- $state[18]);
		s!(s0 <- $state[19]);
		s!(s1 <- $state[20]);
		s!(s2 <- $state[21]);
		s!(s3 <- $state[22]);
		s!(s4 <- $state[23]);
		s!(s5 <- $state[24]);
		s!(s6 <- $state[25]);
		s!(s7 <- $state[26]);
		s!(s8 <- $state[27]);
		s!(s9 <- $state[28]);
		s!(s10 <- $state[29]);
		s!(s11 <- $state[30]);
	};
}

/// Save/load the program counter from/into the given register
pub macro_rules! pc {
	($reg:ident -> $state:ident) => {
		s!($reg -> $state[31]);
	};
	($reg:ident <- $state:ident) => {
		s!($reg <- $state[31]);
	};
}

/// Save/load all the floating point registers. Normally used for context switching.
pub macro_rules! float {
	(save $reg:literal) => {
		s!(f0 -> $state[32]);
		s!(f1 -> $state[33]);
		s!(f2 -> $state[34]);
		s!(f3 -> $state[35]);
		s!(f4 -> $state[36]);
		s!(f5 -> $state[37]);
		s!(f6 -> $state[38]);
		s!(f7 -> $state[39]);
		s!(f8 -> $state[40]);
		s!(f9 -> $state[41]);
		s!(f10 -> $state[42]);
		s!(f11 -> $state[43]);
		s!(f12 -> $state[44]);
		s!(f13 -> $state[45]);
		s!(f14 -> $state[46]);
		s!(f15 -> $state[47]);
		s!(f16 -> $state[48]);
		s!(f17 -> $state[49]);
		s!(f18 -> $state[50]);
		s!(f10 -> $state[51]);
		s!(f20 -> $state[52]);
		s!(f21 -> $state[53]);
		s!(f22 -> $state[54]);
		s!(f23 -> $state[55]);
		s!(f24 -> $state[56]);
		s!(f25 -> $state[57]);
		s!(f26 -> $state[58]);
		s!(f27 -> $state[59]);
		s!(f28 -> $state[60]);
		s!(f29 -> $state[61]);
		s!(f30 -> $state[62]);
		s!(f31 -> $state[63]);
	};
	(load $reg:literal) => {
		s!(f0 <- $state[32]);
		s!(f1 <- $state[33]);
		s!(f2 <- $state[34]);
		s!(f3 <- $state[35]);
		s!(f4 <- $state[36]);
		s!(f5 <- $state[37]);
		s!(f6 <- $state[38]);
		s!(f7 <- $state[39]);
		s!(f8 <- $state[40]);
		s!(f9 <- $state[41]);
		s!(f10 <- $state[42]);
		s!(f11 <- $state[43]);
		s!(f12 <- $state[44]);
		s!(f13 <- $state[45]);
		s!(f14 <- $state[46]);
		s!(f15 <- $state[47]);
		s!(f16 <- $state[48]);
		s!(f17 <- $state[49]);
		s!(f18 <- $state[50]);
		s!(f10 <- $state[51]);
		s!(f20 <- $state[52]);
		s!(f21 <- $state[53]);
		s!(f22 <- $state[54]);
		s!(f23 <- $state[55]);
		s!(f24 <- $state[56]);
		s!(f25 <- $state[57]);
		s!(f26 <- $state[58]);
		s!(f27 <- $state[59]);
		s!(f28 <- $state[60]);
		s!(f29 <- $state[61]);
		s!(f30 <- $state[62]);
		s!(f31 <- $state[63]);
	};
}
