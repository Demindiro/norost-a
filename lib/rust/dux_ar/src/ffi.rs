//! # Types that should be in `core::ffi` but aren't.

pub use core::ffi::*;

#[allow(non_camel_case_types)]
pub type c_int = i32;
