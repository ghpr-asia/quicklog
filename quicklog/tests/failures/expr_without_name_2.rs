use quicklog::info;

fn main() {
    // valid alternatives:
    // info!(name = &x);
    // info!("{}", &x);
    let x = 5;
    info!(&x);
}
