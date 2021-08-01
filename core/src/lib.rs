#![allow(unused_parens)]

#[macro_use]
extern crate num_derive;

// Add hooks for clients to add debug loggers
// eg server logs to stdout and wasm logs to js console
static mut DEBUG_LOGGER : Option<Box<dyn DebugLogger>> = None;

pub fn set_debug_logger(logger : Box<dyn DebugLogger>) {
    // Only should be called once by main thread at init
    unsafe {
        DEBUG_LOGGER = Some(logger);
    }
}

fn debug_logline(logline : &str)
{
    unsafe { DEBUG_LOGGER.as_ref().map(|x| x.log(logline)); }
}

pub trait DebugLogger {
    fn log(&self, logline: &str);
}

pub struct StdoutLogger();

impl DebugLogger for StdoutLogger {
    fn log(&self, logline: &str) {
        println!("{}", logline);
    }
}

macro_rules! debug_log {
    ( $( $t:tt )* ) => {
        crate::debug_logline(&format!( $( $t )* ));
    }
}

pub mod game;
pub mod player_id_map;
pub mod interop;
pub mod timeline;
pub mod crossy_ruleset;
pub mod map;
pub mod rng;

pub use game::*;