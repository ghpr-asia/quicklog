use quicklog::{debug, error, info, init, trace, warn, with_flush};
use quicklog_flush::noop_flusher::NoopFlusher;

fn main() {
    init!();
    with_flush!(NoopFlusher);

    trace!("hello world");
    debug!("hello world");
    info!("hello world");
    warn!("hello world");
    error!("hello world");
}
