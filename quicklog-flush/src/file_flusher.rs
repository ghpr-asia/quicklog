use std::{
    fs::OpenOptions,
    io::{LineWriter, Write},
};

use crate::Flush;

/// Flushes into a file
pub struct FileFlusher(&'static str);

impl FileFlusher {
    /// Flushes into file with specified path. Ensure that the directory exists for the destination log file,
    /// otherwise, an error would be thrown
    pub fn new(path: &'static str) -> FileFlusher {
        FileFlusher(path)
    }
}

impl Flush for FileFlusher {
    fn flush_one(&mut self, display: String) {
        match OpenOptions::new().create(true).append(true).open(self.0) {
            Ok(file) => {
                let mut writer = LineWriter::new(file);
                match writer.write_all(display.as_bytes()) {
                    Ok(_) => (),
                    Err(_) => panic!("Unable to write to file"),
                };
            }
            Err(_) => panic!("Unable to open file"),
        }
    }
}
