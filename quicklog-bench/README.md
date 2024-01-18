# Overview

The benchmarks in this folder mainly test the implementation of our `Serialize` trait against other measures/libraries. In particular, [`serialize_benchmark.rs`](benches/serialize_benchmark.rs) and [`quicklog_benchmark.rs`](benches/quicklog_benchmark.rs) contain the benchmarks shown in the top-level [`README.md`](../README.md#benchmarks).

To run the benchmarks, the general command is `cargo bench --bench <benchmark_name>`. For instance:

```bash
# basic run
cargo bench --bench serialize_benchmark

# run with reduced warmup + measurement time of 1s
cargo bench --bench serialize_benchmark -- --measurement-time 1 --warm-up-time 1
```

If running the benchmarks in the parent (workspace) directory, pass `quicklog-bench` as the target package for `cargo bench`:

```bash
# basic run -- from parent workspace
cargo bench -p quicklog-bench --bench serialize_benchmark

# run with reduced warmup + measurement time of 1s -- from parent workspace
cargo bench -p quicklog-bench --bench serialize_benchmark -- --measurement-time 1 --warm-up-time 1
```
