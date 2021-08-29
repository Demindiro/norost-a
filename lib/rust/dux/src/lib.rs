//! # Dux Standard Rust Library
//!
//! This library defines common types used in the Dux operating system.

#![no_std]
#![feature(const_option)]
#![feature(const_ptr_is_null)]
#![feature(const_ptr_offset)]
#![feature(const_raw_ptr_deref)]
#![feature(global_asm)]

pub mod ipc;
pub mod mem;
pub mod page;
pub mod task;

mod util;

pub use mem::init;
pub use page::{Page, RWX};
