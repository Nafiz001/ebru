//! Replays a print's recipe natively, printing the checksum after every op:
//! `cargo run -p marbling --release --features serde --example replay -- recipe.json`.
//! Compare with the same replay in WebAssembly to find where two platforms part.

#[cfg(feature = "serde")]
fn main() {
    #[derive(serde::Deserialize)]
    struct Recipe {
        ground: u8,
        ops: Vec<marbling::Op>,
    }
    let path = std::env::args().nth(1).expect("a recipe.json");
    let recipe: Recipe = serde_json::from_slice(&std::fs::read(path).expect("readable")).expect("a recipe");
    let mut tray = marbling::Tray::new(recipe.ground);
    for (i, op) in recipe.ops.iter().enumerate() {
        tray.apply(op);
        println!("{i} {:016x}", tray.checksum());
    }
}

#[cfg(not(feature = "serde"))]
fn main() {
    eprintln!("build with --features serde");
}
