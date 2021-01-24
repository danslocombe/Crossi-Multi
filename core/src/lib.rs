#![allow(unused_parens)]

#[macro_use]
extern crate num_derive;

pub mod game;
pub mod interop;
pub mod client;

pub const STATIC_LAG : u32 = 50 * 1000;

use num_traits::FromPrimitive;
pub use game::*;