use quicklog::info;

fn main() {
    // valid alternatives:
    // info!(a = 5, b = 1 + 2);
    // info!("{} {}", 5, 1 + 2);
    info!(5, 1 + 2);
}
