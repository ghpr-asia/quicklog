use quicklog::{debug, error, flush_all, info, init, trace, warn, with_flush};
use quicklog_flush::stdout_flusher::StdoutFlusher;

#[derive(Clone)]
struct S {
    i: u32,
}

impl std::fmt::Display for S {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.i))
    }
}

fn main() {
    init!();
    with_flush!(StdoutFlusher);

    trace!("hello world! {} {} {}", 2, 3, 4);
    trace!("hello, world");
    debug!("hello world! {}", 2);
    info!("hello world! {}", 2);
    warn!("hello world! {}", 2);
    error!("hello world! {}", 2);

    let mut s_0 = S { i: 0 };
    let s_1 = S { i: 1 };
    let s_2 = S { i: 2 };
    let s_3 = S { i: 3 };
    let s_4 = S { i: 4 };
    let s_5 = S { i: 5 };
    let s_6 = S { i: 6 };
    let s_7 = S { i: 7 };
    let s_8 = S { i: 8 };
    let s_9 = S { i: 9 };
    let s_10 = S { i: 10 };

    info!(
        "{} {} {} {} {} {} {} {} {} {} {}",
        s_0, s_1, s_2, s_3, s_4, s_5, s_6, s_7, s_8, s_9, s_10
    );

    s_0.i = 42;

    info!(
        "{} {} {} {} {} {} {} {} {} {} {}",
        s_0, s_1, s_2, s_3, s_4, s_5, s_6, s_7, s_8, s_9, s_10
    );

    flush_all!();
}
