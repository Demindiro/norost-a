//! # 2D commands

pub mod resource;

mod display_info;
mod edid;
mod rect;
mod set_scanout;
mod transfer_to_host_2d;

pub use rect::Rect;
pub use set_scanout::SetScanout;
pub use transfer_to_host_2d::TransferToHost2D;

use crate::ControlHeader;
use simple_endian::{u32le, u64le};
