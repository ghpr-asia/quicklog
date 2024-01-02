# Examples

The examples in this directory showcase distinct features of `quicklog`:

- `defer.rs`: Demonstrates how to do logging with deferred commits (i.e. not making the logs visible via `flush!` immediately), which can slightly improve performance.
- `filter.rs`: Demonstrates how to perform log filtering at runtime.
- `flush_file.rs`: Demonstrates how to change the logging destination to a user-specified file.
- `json.rs`: Demonstrates the two methods of enabling JSON formatting (globally and on a per-log basis).
- `macros`: Basic usage of `quicklog` logging macros and how they can be an almost drop-in replacement for `tracing`, `log`.
- `serialize`: Demonstrates how to do fast logging using our `Serialize` trait (through both deriving `Serializing` on a type and implementing it manually).
