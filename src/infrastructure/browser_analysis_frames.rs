//! Worker-owned frame buffers; only the host coordinator writes native stdout.
use serde_json::Value;
use std::io::{self, Write};
use std::sync::mpsc::SyncSender;

pub(super) enum AnalysisEvent {
    Frame(Vec<u8>),
    Finished(&'static str, &'static str, Result<Value, String>),
}

pub(super) struct AnalysisFrameWriter {
    sender: SyncSender<AnalysisEvent>,
    frame: Vec<u8>,
}

impl AnalysisFrameWriter {
    pub fn new(sender: SyncSender<AnalysisEvent>) -> Self {
        Self {
            sender,
            frame: Vec::new(),
        }
    }
}

impl Write for AnalysisFrameWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.frame.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if !self.frame.is_empty() {
            self.sender
                .send(AnalysisEvent::Frame(std::mem::take(&mut self.frame)))
                .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "Analysis output closed"))?;
        }
        Ok(())
    }
}
