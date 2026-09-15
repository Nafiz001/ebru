//! Marbles a tray the way the workshop does when nobody's at it and prints
//! each stage: `cargo run -p ebru-server --release --example compose -- out/`.

use std::time::Instant;

use ebru_server::patterns::{Composition, Tally};
use ebru_server::print;
use ebru_server::random::Random;
use marbling::Tray;

fn main() {
    let out = std::path::PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "compositions".into()));
    std::fs::create_dir_all(&out).expect("output directory");
    let seed = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(7);
    let mut random = Random::new(seed);
    let mut tray = Tray::new(3);
    let mut tally = Tally::default();

    for stage in 0..3 {
        let plan = loop {
            let plan = Composition::plan(&tray, &mut random);
            // Show one of each movement after the drops.
            if stage == 0 || plan.pattern != "Battal" {
                break plan;
            }
        };
        let start = Instant::now();
        let count = plan.ops.len();
        for op in &plan.ops {
            tally.count(op, &tray);
            tray.apply(op);
        }
        let file = out.join(format!("{stage}-{}.png", plan.pattern.replace(' ', "-")));
        std::fs::write(&file, print::png(&tray, 1200)).expect("write print");
        println!(
            "{:<14} {count:>4} ops in {:>8.1?}, {:>7} points -> {}",
            plan.pattern,
            start.elapsed(),
            tray.points(),
            file.display()
        );
    }
    println!("named: {}", tally.pattern());
}
