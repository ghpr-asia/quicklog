# Quicklog

Fast single-threaded logging framework.

Supports standard logging macros like `trace!`, `debug!`, `info!`, `warn!` and `error!`.

Flushing is deferred until `flush!()` macro is called.

## Objectives

- Deferred Formatting
- Deferred I/O for logging
- Minimise heap allocations
- Low call site latency for logging

## Usage

### Quick Start

```rust
use quicklog::{info, init, flush};

fn main() {
    // initialize required resources
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

### Utilizing different flushing mechanisms

```rust
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

## Benchmark

### Logging a vector of 10 structs

```bash
Loggers/bench log Nested
                        time:   [109.37 ns 110.42 ns 111.57 ns]
Loggers/bench tracing Nested
                        time:   [20.437 µs 20.518 µs 20.600 µs]
Loggers/bench delog Nested
                        time:   [21.008 µs 21.066 µs 21.128 µs]
```

## Support

Please post your bug reports or feature requests on [Github Issues](https://github.com/ghpr-asia/quicklog/issues).

## Roadmap

- [] add single-threaded and multi-threaded variants
- [] Try to remove nested `lazy_format` in recursion
- [] Check number of copies of data made in each log line and possibly reduce it
- [] Review uses of unsafe code
- [] Benchmark multi-threaded performance
- [] Statically assert that strings inside Level and LevelFilter are the same size

## Contributing

We are open for contributions!

## Authors and acknowledgment

[Zack Ng](https://github.com/nhzaci), Tien Dat Nguyen, Michiel van Slobbe, Dheeraj Oswal

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
