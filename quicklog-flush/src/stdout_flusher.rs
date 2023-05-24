use crate::Flush;

/// Flushes into stdout
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
    fn flush_one(&mut self, display: String) {
        print!("{}", display);
    }
}
