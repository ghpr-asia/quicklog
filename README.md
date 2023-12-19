# quicklog

## Overview

`quicklog` is a fast single-threaded logging library. It supports standard logging macros such as `trace!`, `debug!`, `info!`, `warn!` and `error!`, similar to the API exposed by the [`log`](https://docs.rs/log/latest/log/) crate. One key difference is the ability to opt-in [much faster, low-latency logging](#fast-logging).

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
quicklog = "0.3"
```

### Minimum Supported Rust Version

`quicklog` is built against [Rust 1.72.0](https://github.com/rust-lang/rust/releases/tag/1.72.0). Support for older Rust versions is not guaranteed, and it is encouraged that users stick to this or newer versions.

## Usage

### Basic logging

One of the goals of `quicklog` is to provide basic logging functionality similar to `log` or `tracing`'s API. The five core logging macros are supported: `trace!`, `debug!`, `info!`, `warn!` and `error!`. [Structured logging](https://docs.rs/tracing/latest/tracing/#recording-fields) and [named parameters](https://doc.rust-lang.org/std/fmt/#named-parameters) are also supported:

```rust
use quicklog::{info, init, debug, error};

fn main() {
    // initialize required resources. by default, all logs are
    // flushed to stdout.
    init!();

    // basic usage
    info!("Simple format string without arguments");
    info!("Format string with arguments: {:?} {}", "hello world", 123);

    // structured fields -- follows similar rules to `tracing`.
    info!(field_a = 123, field_b = "some text", "Structured fields: {:?}", 99);
    info!(field_a = ?vec![1, 2, 3], field_b = %123, "Structured fields with sigils");

    // named parameters
    let some_var = 10;
    info!("Explicit capture of some_var: {some_var}", some_var = some_var);
    info!("Implicit capture of some_var: {some_var}");

    // flushes everything in queue
    while let Ok(()) = flush!() {}
}
```

As seen in the example, one key step is the need to call `flush!`, which will output a _single_ log entry to stdout by default. This is intentional to avoid potentially expensive I/O operations on performance-critical paths. Also note some [differences with `tracing`](#why-not-quicklog) for basic logging usage.

### Fast logging

`quicklog` provides a `Serialize` trait which is used to opt into fast logging. Applications looking to speed up logging should look to derive a `Serialize` implementation for user-defined types, or provide a manual implementation (see the [serialize example](quicklog/examples/serialize.rs) for a more comprehensively documented tutorial).

After implementing `Serialize` for user-defined types, there are two ways to enable `quicklog` to use them:

1. Place the argument before the format string, as part of the _structured fields_ (no prefix sigil is needed, unlike `?` and `%`). `quicklog` will automatically try to use the `Serialize` implementation for an argument placed in this position.
2. Use the `{:^}` formatting specifier in the format string, similar to how `{:?}` and `{}` are used for arguments implementing the `Debug` and `Display` traits respectively.

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

    // fast -- uses `Serialize`
    info!(s, "fast logging, using Serialize");
    // structured field
    info!(serialize_struct = s, "fast logging, using Serialize");
    // format specifier
    info!("fast logging, using Serialize: serialize_struct={:^}", s);

    // `quicklog` provides default implementations of `Serialize` for
    // certain common data types
    info!(a = 1, b = 2, c = "hello world".to_string(), "fast logging, using default Serialize");

    while let Ok(()) = flush!() {}
}
```

#### Caveats

Due to some constraints, mixing of `Serialize` and `Debug`/`Display` format specifiers in the format string is prohibited. For instance, this will fail to compile:

```rust
// mixing {:^} with {:?} or {} not allowed!
info!("hello world {:^} {:?} {}", 1, 2, 3);
```

However, one can mix-and-match these arguments in the _structured fields_, for example:

```rust
info!(debug = ?some_debug_struct, display = %some_display_struct, serialize = serialize_struct, "serialize args in fmt str: {:^} {:^}", 3, 5);
```

In general, for best performance, try to avoid mixing `Serialize` and non-`Serialize` arguments in each logging call. For instance, try to ensure that on performance-critical paths, every logging argument implements `Serialize`:

```rust
info!(a = 1, b = "hello world", c = 930.123, "Some message: {:^}", some_serialize_struct);
```

### More examples

More usage examples are available [here](quicklog/examples). Some notable ones are:

- [`macros`](quicklog/examples/macros.rs): More comprehensive example of the syntax accepted by our logging macros.
- [`serialize`](quicklog/examples/serialize.rs): Example on implementing `Serialize`, our core trait. Having a manual `Serialize` implementation can be useful at times, usually when some information about the user-defined type can be exploited to squeeze out slightly more performance.

## Advanced usage

Some other potentially useful features supported include:

- Customizing log output location and format
- Compile-time log filtering
- JSON logging
- Deferred logging
- Configuration of max logging capacity

For these advanced features, please refer to the [latest crate documentation](https://docs.rs/quicklog/latest/quicklog/) for full details.

## Benchmarks

Benchmarks were run on an M1 Pro (2021), 16GB RAM setup.

### Logging Integers

`quicklog::info!(a = 1u32, b = 2u32, c = 3u32, "Some data:")`

```bash
Serialize/3x4B           time:   [9.5996 ns 9.6222 ns 9.6417 ns]
```

### Logging Integers + String

`quicklog::info!(a = 1u32, b = 2u32, c = "The quick brown fox jumps over the lazy dog", "Some data:")`

```bash
Serialize/2x4B + string  time:   [10.688 ns 10.701 ns 10.715 ns]
```

### Logging 64B-4KB structs

```bash
Serialize/64B           time:   [10.706 ns 10.717 ns 10.730 ns]

Serialize/128B          time:   [10.889 ns 10.919 ns 10.961 ns]

Serialize/256B          time:   [13.113 ns 13.171 ns 13.239 ns]

Serialize/512B          time:   [19.125 ns 19.509 ns 19.931 ns]

Serialize/1024B         time:   [29.335 ns 29.377 ns 29.414 ns]

Serialize/4KB:          time:   [96.089 ns 96.186 ns 96.316 ns]

tracing/4KB:            time:   [19.677 µs 19.727 µs 19.776 µs]

delog/4KB:              time:   [19.658 µs 19.693 µs 19.734 µs]
```

Full benchmarks can be found in the [benchmarks folder](quicklog/benches).

## Why _not_ `quicklog`?

`quicklog` is still in heavy development and lacks many features supported by e.g. [`tracing`](https://docs.rs/tracing/latest/tracing/), arguably the de facto crate for logging. For instance, `quicklog` currently lacks support for:

- Named targets within the logging macro, e.g.`info!(target: "my_context", ...)`.
- [Spans](https://docs.rs/tracing/latest/tracing/#spans) and logging in asynchronous contexts.
- Integration with certain third-party crates, e.g. through `tracing-subscriber`.

On the whole, it would be good to consider if the extra performance provided by `quicklog` is worth missing out on these features. If these features are important to you, `tracing` and other similar options would be great! Otherwise, `quicklog` aims to still provide the basic logging functionality of these crates while providing the ability to vastly improve logging latency on an opt-in basis.

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
