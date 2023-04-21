use quicklog::{debug, error, flush, info, trace, warn, with_flush};
use quicklog_flush::stdout_flusher::StdoutFlusher;

fn main() {
    with_flush!(StdoutFlusher);

    trace!("hello world! {} {} {}", 2, 3, 4);
    trace!("hello, world");
    debug!("hello world! {}", 2);
    info!("hello world! {}", 2);
    warn!("hello world! {}", 2);
    error!("hello world! {}", 2);

    flush!();
}
