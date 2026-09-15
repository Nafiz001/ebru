//! A rough timing of a busy tray: `cargo run -p marbling --release --example bench`.
use std::time::Instant;

use marbling::{Op, Tray};

fn main() {
    let start = Instant::now();
    let mut tray = Tray::new(1);
    for i in 0..120 {
        let a = i as f64 * 0.37;
        let op = Op::Drop {
            x: 800.0 + (260.0 * a.cos()) * (i as f64 / 120.0),
            y: 500.0 + (200.0 * a.sin()) * (i as f64 / 120.0),
            radius: 18.0 + (i % 7) as f64 * 8.0,
            pigment: (i % 10) as u8,
        };
        tray.apply(&op);
    }
    let drops = start.elapsed();
    println!("120 drops: {:?}, {} points", drops, tray.points());

    let start = Instant::now();
    let mut steps = 0;
    for pass in 0..4 {
        for i in 0..20 {
            let y = 80.0 + i as f64 * 45.0;
            let (x0, x1): (f64, f64) = if pass % 2 == 0 { (100.0, 1500.0) } else { (1500.0, 100.0) };
            let mut x = x0;
            while (x1 - x) * (x1 - x0) > 0.0 {
                let next = if x1 > x0 { (x + 30.0).min(x1) } else { (x - 30.0).max(x1) };
                let op = Op::Rake { from: [x, y], to: [next, y], tines: 1, spacing: 20.0, reach: 8.0 };
                steps += op.substeps();
                tray.apply(&op);
                x = next;
            }
        }
    }
    let rakes = start.elapsed();
    println!(
        "gel-git, 4 passes of 20 strokes ({} substeps): {:?}, {:.2} ms per substep, {} points",
        steps,
        rakes,
        rakes.as_secs_f64() * 1000.0 / steps as f64,
        tray.points()
    );
}
