use quicklog::info;

fn main() {
    let x = 5;
    let y = 6;
    info!(x, ?y "hello world");
}
