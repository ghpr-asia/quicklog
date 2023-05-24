use crate::Flush;

pub struct NoopFlusher;

impl NoopFlusher {
    pub fn new() -> NoopFlusher {
        NoopFlusher {}
    }
}

impl Default for NoopFlusher {
    fn default() -> Self {
        Self::new()
    }
}

impl Flush for NoopFlusher {
    fn flush_one(&mut self, _display: String) {}
}
