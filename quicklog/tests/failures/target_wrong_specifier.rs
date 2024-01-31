use quicklog::info;

fn main() {
    // only accept `target: ...`
    info!(non_target: "my_module", "Hello world");
}
