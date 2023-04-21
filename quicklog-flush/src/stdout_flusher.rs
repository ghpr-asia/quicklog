use crate::Flush;

pub struct StdoutFlusher;

impl StdoutFlusher {
    pub fn new() -> StdoutFlusher {
        StdoutFlusher {}
    }
}

impl Default for StdoutFlusher {
    fn default() -> Self {
        Self::new()
    }
}

impl Flush for StdoutFlusher {
    fn flush(&self, display: String) {
        print!("{}", display);
    }
}
