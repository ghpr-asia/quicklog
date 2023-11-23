use quicklog::{
    debug, error, flush, info, init,
    queue::Metadata,
    serialize::{Serialize, Store},
    trace, warn, with_flush, with_formatter, PatternFormatter,
};
use quicklog_flush::stdout_flusher::StdoutFlusher;

#[derive(Clone, Debug)]
struct S {
    i: i32,
}

impl std::fmt::Display for S {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.i))
    }
}

impl Serialize for S {
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> (Store<'buf>, &'buf mut [u8]) {
        self.i.encode(write_buf)
    }

    fn decode(read_buf: &[u8]) -> (String, &[u8]) {
        i32::decode(read_buf)
    }

    fn buffer_size_required(&self) -> usize {
        self.i.buffer_size_required()
    }
}

struct CustomFormatter;

impl PatternFormatter for CustomFormatter {
    fn custom_format(
        &mut self,
        time: chrono::DateTime<chrono::Utc>,
        metadata: &Metadata,
        log_record: &str,
    ) -> String {
        format!(
            "[{:?}][{}][{}][{}]{}\n",
            time, metadata.file, metadata.line, metadata.level, log_record,
        )
    }
}

fn main() {
    init!();
    with_flush!(StdoutFlusher);
    with_formatter!(CustomFormatter);

    trace!("hello world! {} {} {}", 2, 3, 4);
    trace!("hello, world");
    debug!("hello world! {}", 2);
    info!("hello world! {}", 2);
    warn!("hello world! {}", 2);
    error!("hello world! {}", 2);

    let mut s_0 = S { i: 0 };
    let mut s_1 = S { i: 1 };
    let mut s_2 = S { i: 2 };
    let s_3 = S { i: 3 };
    let s_4 = S { i: 4 };
    let s_5 = S { i: 5 };
    let s_6 = S { i: 6 };
    let s_7 = S { i: 7 };
    let s_8 = S { i: 8 };
    let s_9 = S { i: 9 };
    let s_10 = S { i: 10 };

    // Logging multiple structs
    info!(
        "{} {} {} {} {} {} {} {} {} {} {}",
        s_0, s_1, s_2, s_3, s_4, s_5, s_6, s_7, s_8, s_9, s_10
    );

    s_0.i = 42;
    s_1.i = 420;
    s_2.i = 4200;

    // Logging mutated structs -- copies and captures new data
    info!(
        "{} {} {} {} {} {} {} {} {} {} {}",
        s_0, s_1, s_2, s_3, s_4, s_5, s_6, s_7, s_8, s_9, s_10
    );

    // Debug information
    info!(?s_0);

    // Debug information with custom name
    info!(my_struct = ?s_0);

    // Debug, display, serialize all together
    info!(debug_impl = ?s_0, display_impl = %s_0, serialize_impl = s_0);

    // Debug/display/serialize with format string
    info!(debug_impl = ?s_0, "Display struct: {}", s_0);
    info!(display_impl = %s_0, "Debug and display structs: {:?} {};", s_0, s_0);

    // Named parameters
    info!(debug_impl = ?s_0, "My struct {a}", a = s_0);
    info!(debug_impl = ?s_0, "My struct {s_0:?}");

    while let Ok(()) = flush!() {}
}
