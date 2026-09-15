//! The browser keeps its own copy of the pigments so it can draw before the
//! server answers. This keeps that copy honest.

use marbling::{GROUNDS, PIGMENTS};

#[test]
fn the_web_app_lists_the_same_pigments_as_the_tray() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../web/src/lib/pigments.ts");
    let source = std::fs::read_to_string(path).expect("web/src/lib/pigments.ts");
    for (i, pigment) in PIGMENTS.iter().enumerate() {
        let expected = format!(
            "{{ index: {i}, name: '{}', hex: '{}', ground: {} }}",
            pigment.name,
            pigment.hex(),
            GROUNDS.contains(&(i as u8))
        );
        assert!(source.contains(&expected), "pigments.ts is missing or disagrees on: {expected}");
    }
}
