use std::{
    fs::OpenOptions,
    io::{LineWriter, Write},
};

use crate::Flush;

pub struct FileFlusher(&'static str);

impl FileFlusher {
    pub fn new(name: &'static str) -> FileFlusher {
        FileFlusher(name)
    }
}

impl Flush for FileFlusher {
    fn flush(&mut self, display: String) {
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
