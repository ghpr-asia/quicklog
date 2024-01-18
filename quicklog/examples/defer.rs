use quicklog::{
    commit, commit_on_scope_end, debug_defer, error_defer, flush, info, info_defer, init,
    FlushError, Serialize,
};

#[derive(Debug, PartialEq, Eq)]
struct FooError;

#[derive(Clone, Debug, Serialize)]
struct S {
    i: i32,
}

fn some_computation_with_auto_commit(value: i32) -> Result<(), FooError> {
    let s = S { i: value };
    // Ensures that `commit!` is called after the function returns, regardless
    // of which codepath is taken
    commit_on_scope_end!();

    info_defer!("This should be visible after this function: {:^}", s);

    // hot path computations
    // ...

    if value < 10 {
        return Err(FooError);
    }

    Ok(())
}

fn some_computation_without_auto_commit(value: i32) -> Result<(), FooError> {
    let s = S { i: value };

    info_defer!("This should be visible after this function: {:^}", s);

    // hot path computations
    // ...

    if value < 10 {
        return Err(FooError);
    }

    // We may not reach this commit! It might be better to just `commit!`
    // outside of the function once rather than littering `commit!`s in every
    // possible codepath.
    //
    // If we absolutely *must* see the result of `info_defer` before the
    // function returns, consider using `commit_on_scope_end!`, as shown above.
    commit!();
    Ok(())
}

// To get more control over logging operations, one can opt for the deferred
// logging macros. This holds off on releasing the written slots to be available
// for reading by the reader immediately, saving on a (somewhat expensive)
// commit operation per log.
fn main() {
    init!();

    let s_0 = S { i: 0 };

    info_defer!(a = s_0, "Hello");

    // No results visible yet!
    assert_eq!(flush!(), Err(FlushError::Empty));

    // Do a few more logs
    debug_defer!(debug = s_0, "Debug");
    error_defer!(err = s_0, "Error");

    // Still no results
    assert_eq!(flush!(), Err(FlushError::Empty));

    // Finally, commit whatever we have logged so far to make available for
    // reading
    commit!();
    while let Ok(()) = flush!() {}

    // One can mix and match deferred and non-deferred logs as well
    info_defer!(b = s_0, "Hello 2");
    assert_eq!(flush!(), Err(FlushError::Empty));

    // The default logging macros commit by default, so the result of the
    // previous info_defer will become visible
    info!(c = s_0, "Hello 3");

    while let Ok(()) = flush!() {}

    // Calling function that does a deferred log + commit on scope end
    assert_eq!(some_computation_with_auto_commit(5), Err(FooError));
    assert_eq!(flush!(), Ok(()));

    // If the function does not use `commit_on_scope_end!`, then there is a
    // possibility that it will take a codepath which does not call `commit!`.
    assert_eq!(some_computation_without_auto_commit(5), Err(FooError));
    if matches!(flush!(), Err(FlushError::Empty)) {
        // Need to explicitly commit after the function returns, since it exited early
        commit!();
        assert_eq!(flush!(), Ok(()));
    }
}
