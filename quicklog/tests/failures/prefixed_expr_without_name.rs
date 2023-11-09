use quicklog::info;

fn foo() {}

fn main() {
    // valid alternatives:
    // info!(name = ?&x);
    // info!("{:?}", &x);
    let x = 5;
    info!(?&x);
}
