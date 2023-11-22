use quicklog::{
    commit, debug_defer, error_defer, flush, info, info_defer, init, queue::FlushError, try_flush,
    with_flush, Serialize,
};
use quicklog_flush::stdout_flusher::StdoutFlusher;

#[derive(Clone, Debug, Serialize)]
struct S {
    i: i32,
}

// To get more control over logging operations, one can opt for the deferred
// logging macros. This holds off on releasing the written slots to be available
// for reading by the reader immediately, saving on a (somewhat expensive)
// commit operation per log.
fn main() {
    init!();
    with_flush!(StdoutFlusher);

    let s_0 = S { i: 0 };

    info_defer!(a = s_0, "Hello");

    // No results visible yet!
    assert_eq!(try_flush!(), Err(FlushError::Empty));

    // Do a few more logs
    debug_defer!(debug = s_0, "Debug");
    error_defer!(err = s_0, "Error");

    // Still no results
    assert_eq!(try_flush!(), Err(FlushError::Empty));

    // Finally, commit whatever we have logged so far to make available for
    // reading
    commit!();
    flush!();

    // One can mix and match deferred and non-deferred logs as well
    info_defer!(b = s_0, "Hello 2");
    assert_eq!(try_flush!(), Err(FlushError::Empty));

    // The default logging macros commit by default, so the result of the
    // previous info_defer will become visible
    info!(c = s_0, "Hello 3");
    flush!();
}
