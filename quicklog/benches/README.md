# Overview

The benchmarks in this folder mainly test the implementation of our `Serialize` trait against other measures/libraries. In particular, [`serialize_benchmark.rs`](serialize_benchmark.rs) and [`quicklog_benchmark.rs`](quicklog_benchmark.rs) contain the benchmarks shown in the top-level [`README.md`](../../README.md#benchmarks).

To run the benchmarks, the general command is `cargo bench --features=bench --bench <benchmark_name>`. For instance:

```bash
# basic run
cargo bench --features=bench --bench serialize_benchmark

# run with reduced warmup + measurement time of 1s
cargo bench --features=bench --bench serialize_benchmark -- --measurement-time 1 --warm-up-time 1
```
