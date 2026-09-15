//! A real print's recipe, 648 ops made by hand on the live tray, replays to
//! the same checksum on every platform. If a compiler, a CPU or a change to
//! the maths shifts a single bit of a single outline, this fails.
//!
//! `web/scripts/determinism.mjs` checks the same recipe in WebAssembly.

use marbling::{Op, Tray};
use serde_json::Value;

const RECIPE: &str = include_str!("fixtures/recipe.json");
const CHECKSUM: u64 = 0x95955355c6e7c422;

fn op(v: &Value) -> Op {
    let f = |key: &str| v[key].as_f64().expect("a number");
    let pair = |key: &str| [v[key][0].as_f64().unwrap(), v[key][1].as_f64().unwrap()];
    match v["op"].as_str().unwrap() {
        "drop" => Op::Drop { x: f("x"), y: f("y"), radius: f("radius"), pigment: f("pigment") as u8 },
        "rake" => Op::Rake {
            from: pair("from"),
            to: pair("to"),
            tines: f("tines") as u8,
            spacing: f("spacing"),
            reach: f("reach"),
        },
        "swirl" => Op::Swirl { x: f("x"), y: f("y"), turn: f("turn"), reach: f("reach") },
        other => panic!("unexpected op {other}"),
    }
}

#[test]
fn a_recorded_recipe_replays_to_its_recorded_checksum() {
    let recipe: Value = serde_json::from_str(RECIPE).unwrap();
    let mut tray = Tray::new(recipe["ground"].as_u64().unwrap() as u8);
    let ops = recipe["ops"].as_array().unwrap();
    assert_eq!(ops.len(), 648);
    for v in ops {
        tray.apply(&op(v));
    }
    assert_eq!(tray.checksum(), CHECKSUM, "got {:016x}", tray.checksum());
}
