use marbling::{Op, OpError, Point, Tray, decode, encode};

fn centroid(points: &[Point]) -> Point {
    let n = points.len() as f64;
    Point { x: points.iter().map(|p| p.x).sum::<f64>() / n, y: points.iter().map(|p| p.y).sum::<f64>() / n }
}

fn mean_radius(points: &[Point], c: Point) -> f64 {
    points.iter().map(|p| ((p.x - c.x).powi(2) + (p.y - c.y).powi(2)).sqrt()).sum::<f64>() / points.len() as f64
}

fn drop(x: f64, y: f64, radius: f64, pigment: u8) -> Op {
    Op::Drop { x, y, radius, pigment }
}

#[test]
fn a_drop_becomes_a_circle_of_its_radius() {
    let mut tray = Tray::new(9);
    tray.apply(&drop(800.0, 500.0, 90.0, 0));
    assert_eq!(tray.inks.len(), 1);
    let ink = &tray.inks[0];
    // Outlines are polygons, so a circle's mean radius sits a hair inside the true one.
    let r = mean_radius(&ink.outline, Point { x: 800.0, y: 500.0 });
    assert!((r - 90.0).abs() < 0.3, "radius {r}");
    let circle = std::f64::consts::PI * 90.0 * 90.0;
    assert!((ink.area().abs() - circle).abs() / circle < 0.01);
}

#[test]
fn a_new_drop_pushes_the_old_ones_out_and_squared_radii_add() {
    // A ring at radius 50 pushed by a drop of 120 at its centre ends at sqrt(50² + 120²) = 130.
    let centre = Point { x: 800.0, y: 500.0 };
    let mut once = Tray::new(9);
    once.apply(&drop(800.0, 500.0, 50.0, 0));
    once.apply(&drop(800.0, 500.0, 120.0, 2));

    let mut twice = Tray::new(9);
    twice.apply(&drop(800.0, 500.0, 50.0, 0));
    twice.apply(&drop(800.0, 500.0, 72.0, 2));
    twice.apply(&drop(800.0, 500.0, 96.0, 4)); // 72² + 96² = 120²

    let r_once = mean_radius(&once.inks[0].outline, centre);
    let r_twice = mean_radius(&twice.inks[0].outline, centre);
    assert!((r_once - 130.0).abs() < 0.3, "{r_once}");
    assert!((r_twice - 130.0).abs() < 0.3, "{r_twice}");
    assert!((r_once - r_twice).abs() < 0.1, "one push and two pushes agree: {r_once} vs {r_twice}");
}

#[test]
fn ink_keeps_its_area_however_it_is_pulled() {
    let mut tray = Tray::new(9);
    tray.apply(&drop(700.0, 500.0, 60.0, 0));
    let before = tray.inks[0].area().abs();
    tray.apply(&drop(790.0, 520.0, 70.0, 2));
    for i in 0..8 {
        let y = 440.0 + i as f64 * 15.0;
        tray.apply(&Op::Rake { from: [560.0, y], to: [780.0, y], tines: 1, spacing: 20.0, reach: 8.0 });
    }
    tray.apply(&Op::Swirl { x: 700.0, y: 500.0, turn: 1.2, reach: 50.0 });
    let after = tray.inks[0].area().abs();
    assert!((after - before).abs() / before < 0.02, "area {before} became {after}");
}

#[test]
fn a_rake_carries_nearby_ink_and_leaves_distant_ink_alone() {
    let mut tray = Tray::new(9);
    tray.apply(&drop(600.0, 300.0, 30.0, 0)); // on the stroke's path
    tray.apply(&drop(600.0, 800.0, 30.0, 2)); // far from it
    let near_before = centroid(&tray.inks[0].outline);
    let far_before = tray.inks[1].outline.clone();

    tray.apply(&Op::Rake { from: [500.0, 300.0], to: [700.0, 300.0], tines: 1, spacing: 20.0, reach: 12.0 });

    // The bath slips past the tine, so ink travels a good way but not the
    // stroke's full 200 units.
    let near_after = centroid(&tray.inks[0].outline);
    let moved = near_after.x - near_before.x;
    assert!(moved > 20.0 && moved < 110.0, "ink on the path moved {moved}");
    assert!((near_after.y - near_before.y).abs() < 2.0);
    assert_eq!(tray.inks[1].outline, far_before);
}

#[test]
fn a_swirl_keeps_every_point_at_its_distance_from_the_centre() {
    let mut tray = Tray::new(9);
    tray.apply(&drop(840.0, 500.0, 40.0, 0));
    let c = Point { x: 800.0, y: 500.0 };
    let distances: Vec<f64> = tray.inks[0].outline.iter().map(|p| ((p.x - c.x).powi(2) + (p.y - c.y).powi(2)).sqrt()).collect();
    tray.apply(&Op::Swirl { x: 800.0, y: 500.0, turn: 1.0, reach: 60.0 });
    let after: Vec<f64> = tray.inks[0].outline.iter().map(|p| ((p.x - c.x).powi(2) + (p.y - c.y).powi(2)).sqrt()).collect();
    // Resampling may add points, but none may leave the band the ink started in.
    let (lo, hi) = distances.iter().fold((f64::MAX, f64::MIN), |(lo, hi), d| (lo.min(*d), hi.max(*d)));
    assert!(after.iter().all(|d| *d > lo - 0.5 && *d < hi + 0.5));
}

#[test]
fn outlines_stay_finely_sampled_as_they_stretch() {
    let mut tray = Tray::new(9);
    tray.apply(&drop(800.0, 500.0, 50.0, 0));
    for i in 0..12 {
        let x = 640.0 + i as f64 * 30.0;
        tray.apply(&Op::Rake { from: [x, 380.0], to: [x, 620.0], tines: 1, spacing: 20.0, reach: 6.0 });
    }
    let outline = &tray.inks[0].outline;
    let longest = outline
        .iter()
        .zip(outline.iter().cycle().skip(1))
        .map(|(a, b)| ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt())
        .fold(0.0, f64::max);
    assert!(longest <= 4.5 + 1e-9, "longest edge {longest}");
}

fn busy_tray() -> Tray {
    let mut tray = Tray::new(1);
    for i in 0..30 {
        let a = i as f64 * 0.7;
        tray.apply(&drop(800.0 + 180.0 * a.cos(), 500.0 + 120.0 * a.sin(), 20.0 + (i % 5) as f64 * 9.0, (i % 10) as u8));
    }
    for i in 0..6 {
        let y = 300.0 + i as f64 * 80.0;
        tray.apply(&Op::Rake { from: [400.0, y], to: [620.0, y], tines: 5, spacing: 30.0, reach: 7.0 });
    }
    tray.apply(&Op::Swirl { x: 820.0, y: 480.0, turn: -1.3, reach: 90.0 });
    tray
}

#[test]
fn the_same_ops_give_the_same_tray_to_the_bit() {
    assert_eq!(busy_tray().checksum(), busy_tray().checksum());
}

#[test]
fn a_restored_snapshot_carries_on_exactly_like_the_original() {
    let mut original = busy_tray();
    let bytes = encode(&original);
    let mut restored = decode(&bytes).expect("decodes");
    assert_eq!(restored.checksum(), original.checksum());
    assert_eq!(restored.seq, original.seq);

    let more = [
        drop(300.0, 200.0, 70.0, 6),
        Op::Rake { from: [200.0, 600.0], to: [420.0, 560.0], tines: 3, spacing: 40.0, reach: 9.0 },
        Op::Swirl { x: 600.0, y: 500.0, turn: 0.8, reach: 120.0 },
    ];
    for op in &more {
        original.apply(op);
        restored.apply(op);
    }
    assert_eq!(restored.checksum(), original.checksum());
}

#[test]
fn stepping_an_op_gives_the_same_tray_as_applying_it() {
    let mut stepped = busy_tray();
    let mut applied = busy_tray();
    let op = Op::Rake { from: [300.0, 520.0], to: [520.0, 480.0], tines: 1, spacing: 20.0, reach: 10.0 };
    applied.apply(&op);
    stepped.enqueue(op);
    while stepped.advance(3) > 0 {}
    assert_eq!(stepped.checksum(), applied.checksum());
    assert!(stepped.is_settled());
}

#[test]
fn a_skim_clears_the_bath() {
    let mut tray = busy_tray();
    let seq = tray.seq;
    tray.apply(&Op::Skim { ground: 4 });
    assert!(tray.inks.is_empty());
    assert_eq!(tray.ground, 4);
    assert_eq!(tray.seq, seq + 1);
}

#[test]
fn broken_snapshots_are_refused() {
    let bytes = encode(&busy_tray());
    assert!(decode(b"nope").is_err());
    assert!(decode(&bytes[..bytes.len() - 7]).is_err());
}

#[test]
fn ops_from_clients_are_checked() {
    assert_eq!(drop(f64::NAN, 1.0, 20.0, 0).validate(), Err(OpError::NotANumber));
    assert_eq!(drop(-500.0, 1.0, 20.0, 0).validate(), Err(OpError::OffTheTray));
    assert_eq!(drop(10.0, 10.0, 900.0, 0).validate(), Err(OpError::OutOfRange("radius")));
    assert_eq!(drop(10.0, 10.0, 20.0, 99).validate(), Err(OpError::NoSuchPigment));
    let long = Op::Rake { from: [0.0, 0.0], to: [900.0, 0.0], tines: 1, spacing: 20.0, reach: 5.0 };
    assert_eq!(long.validate(), Err(OpError::OutOfRange("stroke length")));
    assert!(drop(10.0, 10.0, 20.0, 0).validate().is_ok());
}
