//! # Dux Standard Rust Library
//!
//! This library defines common types used in the Dux operating system.

#![no_std]
#![feature(const_evaluatable_checked)]
#![feature(const_fmt_arguments_new)]
#![feature(const_fn_transmute)]
#![feature(const_generics)]
#![feature(const_option)]
#![feature(const_panic)]
#![feature(const_ptr_is_null)]
#![feature(const_ptr_offset)]
#![feature(const_raw_ptr_deref)]
#![feature(global_asm)]
#![feature(inherent_associated_types)]
#![feature(option_result_unwrap_unchecked)]

pub mod ipc;
pub mod mem;
pub mod page;
pub mod task;

mod util;

pub use page::{Page, RWX};
