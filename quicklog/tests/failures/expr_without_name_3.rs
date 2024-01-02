use quicklog::info;

fn foo() {}

fn main() {
    // valid alternatives:
    // info!(name = foo());
    // info!("{}", foo());
    info!(foo(), some_other = 5);
}
