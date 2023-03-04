use std::fs::{File, OpenOptions};
use std::io::prelude::*;

pub struct TelemetryTracer
{
    file : File,
    event_buffer : Vec<TelemetryEvent>
}

impl TelemetryTracer
{
    pub fn new(filename : &str) -> Self {
        let file = OpenOptions::new().write(true).append(true).create(true).open(filename).unwrap();

        Self {
            file,
            event_buffer: Default::default(),
        }

    }

    pub fn push(&mut self, event : TelemetryEvent) {
        self.event_buffer.push(event);
    }

    pub fn flush(&mut self) {
        if (self.event_buffer.is_empty()) {
            return;
        }

        let mut events = Vec::new();
        std::mem::swap(&mut events, &mut self.event_buffer);

        for event in events.into_iter() {
            writeln!(self.file, "{:?} - {:#?}", event.player_id, event.event).unwrap();
        }
    }
}

#[derive(Debug, Clone)]
pub struct TelemetryEvent
{
    pub player_id : crate::PlayerId,
    pub event : crate::interop::TelemetryMessage
}