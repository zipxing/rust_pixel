// https://users.rust-lang.org/t/if-let-else-if-let-else-better-way-to-structure-code/104393/10
use std::sync::OnceLock;

static WINNER: OnceLock<&str> = OnceLock::new();

fn main() {
    let winner = std::thread::scope(|s| {
        s.spawn(|| WINNER.set("thread"));

        std::thread::yield_now(); // give them a chance...

        WINNER.get_or_init(|| "main")
    });

    println!("{:?} wins!", WINNER.get().unwrap());
}
