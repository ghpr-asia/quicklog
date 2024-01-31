use quicklog::info;

fn main() {
    // missing comma between target specifier and following arguments
    info!(target: "my_module" "Hello world");
}
