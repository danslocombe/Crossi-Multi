#![allow(unused_parens)]

#[macro_use]
extern crate num_derive;

pub mod game;
pub mod player_id_map;
pub mod interop;
pub mod client;
pub mod timeline;
pub mod crossy_ruleset;
pub mod map;
pub mod rng;

pub use game::*;

use std::fs::File;
use std::io::Write;
use std::cell::RefCell;

// This is not actually threadsafe, but we only run the client single threaded
// and want a static instance
unsafe impl Sync for DebugLogger {}

pub struct DebugLogger {
    pub file: Option<RefCell<File>>,
}

static mut DEBUG_LOGGER : DebugLogger = DebugLogger { file : None };

fn debug_log(logline : &str)
{
    unsafe { DEBUG_LOGGER.log(logline); }
}

impl DebugLogger {
    pub fn log(&self, logline: &str) {
        if let Some(logger) = self.file.as_ref() {
            //let mut m = logger.lock().unwrap();
	    let mut m = logger.borrow_mut();
            m.write(logline.as_bytes()).unwrap();
            m.write(b"\n").unwrap();
        }
    }
}