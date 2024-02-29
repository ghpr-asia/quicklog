# quicklog

Fast single-threaded logging framework, almost 200x faster than `tracing` and `delog` for large structs.

Supports standard logging macros like `trace!`, `debug!`, `info!`, `warn!` and `error!`.

Flushing is deferred until `flush!()` macro is called.

## Objectives

- Deferred formatting
- Deferred I/O
- Low call site latency

While `tracing` is a popular library for event tracing and are really good at what they do, `quicklog` is optimized for low callsite latency, paying the cost of formatting and I/O on a separate thread, away from the hot path.

## Installation

#### Install using Cargo

```bash
cargo add quicklog
```

#### Add to Cargo.toml

```toml
# Cargo.toml
[dependencies]
quicklog = "0.1.18"
# ...
```

## Usage

### Quick Start

```rust
use quicklog::{info, init, flush};

fn main() {
    // initialise required resources, called near application entry point
    init!();

    // adds item to logging queue
    info!("hello world");

    let some_var = 10;
    // variables are passed by copy
    info!("value of some_var: {}", some_var);

    // flushes everything in queue
    flush!();
}
```

### Utilising `Serialize`

In order to avoid cloning a large struct, you can implement the `Serialize` trait.

This allows you to copy specific parts of your struct onto a circular byte buffer and avoid copying the rest by encoding providing a function to decode your struct from a byte buffer.

For a complete example, refer to `~/quicklog/benches/logger_benchmark.rs`.

```rust
use quicklog::serialize::{Serialize, Store};

struct SomeStruct {
    num: i64
}

impl Serialize for SomeStruct {
   fn encode(&self, write_buf: &'static mut [u8]) -> Store { /* some impl */ }
   fn buffer_size_required(&self) -> usize { /* some impl */ }
}

fn main() {
    let s = SomeStruct { num: 1_000_000 };
    info!("some struct: {}", ^s);
}
```


### Utilising different flushing mechanisms

```rust
use quicklog_flush::stdout_flusher::StdoutFlusher;
use quicklog::{info, init, flush, with_flush_into_file, with_flush};

fn main() {
    init!();

    // flush into stdout
    with_flush!(StdoutFlusher);

    // item goes into logging queue
    info!("hello world");

    // flushed into stdout
    flush!()

    // flush into a file path specified
    with_flush_into_file!("logs/my_log.log");

    info!("shave yaks");

    // flushed into file
    flush!();
}
```

More usage examples are available [here](quicklog/examples/macros.rs).

## Benchmark

Measurements are made on a 2020 16 core M1 Macbook Air with 16 GB RAM.

### Logging a struct with a vector of 10 large structs

| Logger   | Lower Bound   | Estimate      | Upper Bound   |
| -------- | ------------- | ------------- | ------------- |
| quicklog | **103.76 ns** | **104.14 ns** | **104.53 ns** |
| tracing  | 22.336 µs     | 22.423 µs     | 22.506 µs     |
| delog    | 21.528 µs     | 21.589 µs     | 21.646 µs     |

### Logging a single struct with 100 array elements

| Logger   | Lower Bound   | Estimate      | Upper Bound   |
| -------- | ------------- | ------------- | ------------- |
| quicklog | **61.399 ns** | **62.436 ns** | **63.507 ns** |
| tracing  | 2.6501 µs     | 2.6572 µs     | 2.6646 µs     |
| delog    | 2.7610 µs     | 2.7683 µs     | 2.7761 µs     |

### Logging a small struct with primitives

| Logger   | Lower Bound   | Estimate      | Upper Bound   |
| -------- | ------------- | ------------- | ------------- |
| quicklog | **28.561 ns** | **28.619 ns** | **28.680 ns** |
| tracing  | 627.79 µs     | 629.91 µs     | 632.06 µs     |
| delog    | 719.54 µs     | 721.19 µs     | 722.96 µs     |

## Contribution & Support

We are open to contributions and requests!

Please post your bug reports or feature requests on [Github Issues](https://github.com/ghpr-asia/quicklog/issues).

## Roadmap

- [] add single-threaded and multi-threaded variants
- [] Try to remove nested `lazy_format` in recursion
- [] Check number of copies of data made in each log line and possibly reduce it
- [] Review uses of unsafe code
- [] Benchmark multi-threaded performance
- [] Statically assert that strings inside Level and LevelFilter are the same size

## Authors and acknowledgment

[Zack Ng](https://github.com/nhzaci), Tien Dat Nguyen, Michiel van Slobbe, Dheeraj Oswal

### Crates
- [Lucretiel/lazy_format](https://github.com/Lucretiel/lazy_format)
- [japaric/heapless](https://github.com/japaric/heapless)

### References
- [tokio-rs/tracing](https://github.com/tokio-rs/tracing)
- [trussed-dev/delog](https://github.com/trussed-dev/delog)

## License

Copyright 2023 [Grasshopper Asia](https://github.com/ghpr-asia)

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
