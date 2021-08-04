//! # Dux Standard Rust Library
//!
//! This library defines common types used in the Dux operating system.

#![no_std]
#![feature(global_asm)]
#![feature(const_option)]
#![feature(const_ptr_is_null)]
#![feature(const_raw_ptr_deref)]
#![feature(const_raw_ptr_to_usize_cast)]

pub mod ipc;
pub mod mem;

mod page;

pub use mem::init;
pub use page::Page;
