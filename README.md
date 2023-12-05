# quicklog

Fast single-threaded logging framework.

Supports standard logging macros like `trace!`, `debug!`, `info!`, `warn!` and `error!`.

Flushing is deferred until the `flush!()` macro is called.

## Objectives

- Deferred Formatting
- Deferred I/O for logging
- Minimise heap allocations
- Low call site latency for logging

## Usage

### Quick start

```rust no_run
use quicklog::{info, init, flush, Serialize};

fn main() {
    // initialize required resources. by default, all logs are
    // flushed to stdout
    init!();

    // similar macro syntax as `tracing`, `log`.
    // eager formatting of format string + arguments, by default
    let some_var = 10;
    info!("value of some_var: {}", some_var);
    info!(?some_var, "debug value of some var: ");
    info!(my_var = %some_var, "display value of some named var: ");

    // named parameters are supported
    info!("explicit capture of some_var: {some_var}", some_var = some_var);
    info!("implicit capture of some_var: {some_var}");

    // fast path - arguments implementing `Serialize`.
    // arguments before the format string without either a `?` or `%` prefix
    // will be logged using their `Serialize` implementations, where possible
    info!(a = 5, b = 999, c = "some string", "hello world");

    // flushes everything in queue
    while let Ok(()) = flush!() {}
}
```

### Fast logging

`quicklog` provides a `Serialize` trait which is used to opt into fast logging. Applications looking to speed up logging should look to derive a `Serialize` implementation for user-defined types, or provide a manual implementation (see the [serialize example](quicklog/examples/serialize.rs) for a more comprehensive tutorial).

To allow `quicklog` to use the `Serialize` implementations of the logging arguments, there are two requirements:

- The (optionally-named) argument must be placed _before_ the format string.
- The argument must not have a prefix (`?` or `%`).

```rust no_run
use quicklog::{flush, info, init, Serialize};

// derive `Serialize` macro
#[derive(Debug, Serialize)]
struct Foo {
    a: usize,
    b: String,
    c: Vec<&'static str>
}

fn main() {
    let s = Foo {
        a: 1000,
        b: "hello".to_string(),
        c: vec!["a", "b", "c"]
    };

    init!();

    // slow-by-default -- eager formatting
    info!("eager logging, using Debug: {:?}", s);
    info!(my_struct = ?s, "eager logging, using Debug:");

    // fast -- uses `Serialize`
    info!(s, "fast logging, using Serialize");
    info!(serialize_struct = s, "fast logging, using Serialize");

    // `quicklog` provides default implementations of `Serialize` for
    // certain common data types
    info!(a = 1, b = 2, c = "hello world".to_string(), "fast logging, using default Serialize");

    // flushes everything in queue
    while let Ok(()) = flush!() {}
}
```

### Deferred logging

For more performance-sensitive applications, one can opt for the deferred logging macros: `trace_defer`, `debug_defer`, `info_defer`, `warn_defer` or `error_defer`. These macros accept the same logging syntax as their non-`defer` counterparts, but must be followed by an explicit call to `commit` in order for the logs to become visible via `flush`. This can be helpful when an application makes a series of logging calls consecutively in some kind of event loop, and only needs to flush/make visible those logs after the main events have been processed.

```rust no_run
use quicklog::{commit, flush, info_defer, init};

fn main() {
    init!();

    // log without making data visible immediately
    info_defer!("hello world");
    info_defer!(a = 1, b = 2, "some data");

    // no data committed yet!
    assert!(flush!().is_err());

    // commits all data written so far
    commit!();

    // output of logs should be visible now
    while let Ok(()) = flush!() {}
}
```

### Customizing log output location and format

By default, `quicklog` outputs logs to stdout in the following format: `[utc datetime]"log string"`. For instance:

```rust no_run
use quicklog::Serialize;
#[derive(Serialize)]
struct S {
    i: usize,
}
let some_struct = S { i: 0 };

// [2023-11-29T05:05:39.310212084Z]Some data: a=S { i: 0 }
quicklog::info!(a = some_struct, "Some data:")
```

It is possible to mix-and-match the output location and log format using the `with_flush` and `with_formatter` macros, which take in an implementor of the `Flush` and the `PatternFormatter` traits respectively.

```rust no_run
use quicklog::queue::Metadata;
use quicklog::{DateTime, Utc};
use quicklog::{flush, init, info, with_flush_into_file, with_formatter, PatternFormatter};

pub struct PlainFormatter;

impl PatternFormatter for PlainFormatter {
    fn custom_format(
        &mut self,
        _: DateTime<chrono::Utc>,
        _: &Metadata,
        log_record: &str,
    ) -> String {
        format!("{}\n", log_record)
    }
}

fn main() {
    init!();

    // item goes into logging queue
    info!("hello world");

    // flushed into stdout: [utc datetime]"hello world"
    _ = flush!();

    // change log output format according to `PlainFormatter`
    with_formatter!(PlainFormatter);
    // flush into a file path specified
    with_flush_into_file!("logs/my_log.log");

    info!("shave yaks");

    // flushed into file
    _ = flush!();
}
```

### Configuring max logging queue capacity

By default, `quicklog` uses a queue with a capacity of 1MB to store written messages. To specify a different size, pass the desired size to the `init` macro on first initialization:

```rust no_run
use quicklog::init;

fn main() {
    // 10MB queue
    init!(10 * 1024 * 1024);

    // log some data...
}
```

### More examples

More usage examples are available [here](quicklog/examples). Some notable ones are:

- [`macros`](quicklog/examples/macros.rs): More comprehensive example of the syntax accepted by our logging macros (similar to `tracing`, `log`).
- [`serialize`](quicklog/examples/serialize.rs): Example on implementing `Serialize`, our core trait. Having a manual `Serialize` implementation can be useful when some information about the user-defined type can be exploited to squeeze out slightly more performance.

## Benchmarks

### Logging Integers

`quicklog::info!(a = 1u32, b = 2u32, c = 3u32, "Some data:")`

```bash
Serialize/3x4B           time:   [20.089 ns 20.106 ns 20.123 ns]
```

### Logging Integers + String

`quicklog::info!(a = 1u32, b = 2u32, c = "The quick brown fox jumps over the lazy dog", "Some data:")`

```bash
Serialize/2x4B + string  time:   [25.520 ns 25.563 ns 25.615 ns]
```

### Logging 64B-4KB structs

```bash
Serialize/64B           time:   [12.444 ns 12.464 ns 12.484 ns]

Serialize/128B          time:   [12.309 ns 12.332 ns 12.360 ns]

Serialize/256B          time:   [13.654 ns 13.712 ns 13.777 ns]

Serialize/512B          time:   [24.607 ns 24.698 ns 24.779 ns]

Serialize/1024B         time:   [41.746 ns 42.450 ns 43.000 ns]

Serialize/4KB:          time:   [114.15 ns 114.73 ns 115.45 ns]

tracing/4KB:            time:   [20.985 µs 21.050 µs 21.117 µs]

delog/4KB:              time:   [21.942 µs 21.964 µs 21.991 µs]
```

Full benchmarks can be found in the [benchmarks folder](quicklog/benches).

## Why _not_ `quicklog`?

`quicklog` is still in heavy development and lacks many features supported by e.g. [`tracing`](https://docs.rs/tracing/latest/tracing/), arguably the de facto crate for logging. For instance, `quicklog` currently lacks support for named targets within the logging macro, e.g.`info!(target: "my_context", ...)`. Also, if you require [spans](https://docs.rs/tracing/latest/tracing/#spans), logging in asynchronous contexts or integration with certain third-party crates, `tracing` provides these out-of-the-box with much more customizability.

On the whole, it would be good to consider if the extra performance provided by `quicklog` is worth missing out on these features. If these features are important to you, `tracing`, `log` and other similar options would be great! Otherwise, `quicklog` aims to still provide the basic logging functionality of these crates while providing the capability, on an opt-in basis, to vastly improve logging latency.

## Support

Please post your bug reports or feature requests on [Github Issues](https://github.com/ghpr-asia/quicklog/issues).

## Roadmap

- [] Multi-threaded implementation for logging in background thread
- [] Review uses of unsafe code
- [] Benchmark multi-threaded performance

## Contributing

We are open for contributions!

## Authors and acknowledgment

[Zack Ng](https://github.com/nhzaci), Tien Dat Nguyen, Michiel van Slobbe, Dheeraj Oswal

## License

Copyright 2023 [Grasshopper Asia](https://github.com/ghpr-asia)

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

```ignore
http://www.apache.org/licenses/LICENSE-2.0
```

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
