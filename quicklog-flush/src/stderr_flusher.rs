use crate::Flush;

/// Flushes into stderr
pub struct StderrFlusher;

impl StderrFlusher {
    pub fn new() -> StderrFlusher {
        StderrFlusher {}
    }
}

impl Default for StderrFlusher {
    fn default() -> Self {
        Self::new()
    }
}

impl Flush for StderrFlusher {
    fn flush_one(&mut self, display: String) {
        eprint!("{}", display);
    }
}
