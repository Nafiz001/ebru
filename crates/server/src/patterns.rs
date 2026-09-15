//! Naming patterns, and the tray marbling on its own.
//!
//! Traditional ebru has names for the patterns its movements make. The
//! workshop keeps a tally of what was done to the bath and names each print
//! after the most characteristic thing in it:
//!
//! - **Battal**: drops alone, the first and simplest ebru.
//! - **Hatip**: drops set inside drops, then drawn through with a stylus,
//!   after the eighteenth-century marbler Hatip Mehmed Efendi.
//! - **Gel-git**: "come and go", a stylus drawn back and forth across the tray.
//! - **Taraklı**: combed, a gel-git pulled through with a many-toothed comb.
//! - **Bülbül yuvası**: "nightingale's nest", small spirals turned into the ink.

use marbling::{GROUNDS, HEIGHT, Op, PIGMENTS, Tray, WIDTH};

use crate::random::Random;

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Tally {
    pub drops: u32,
    pub nested_drops: u32,
    pub strokes: u32,
    pub combings: u32,
    pub swirls: u32,
}

impl Tally {
    /// Counts an op, given the tray as it was just before it.
    pub fn count(&mut self, op: &Op, before: &Tray) {
        match *op {
            Op::Drop { x, y, .. } => {
                self.drops += 1;
                if lands_inside_ink(before, x, y) {
                    self.nested_drops += 1;
                }
            }
            Op::Rake { tines, .. } if tines > 1 => self.combings += 1,
            Op::Rake { .. } => self.strokes += 1,
            Op::Swirl { .. } => self.swirls += 1,
            Op::Skim { .. } => *self = Tally::default(),
        }
    }

    pub fn ops(&self) -> u32 {
        self.drops + self.strokes + self.combings + self.swirls
    }

    /// The traditional name for what's on the tray.
    pub fn pattern(&self) -> &'static str {
        if self.swirls >= 8 {
            "Bülbül yuvası"
        } else if self.combings >= 6 {
            "Taraklı"
        } else if self.nested_drops >= 4 && self.strokes >= 3 {
            "Hatip"
        } else if self.strokes >= 12 {
            "Gel-git"
        } else if self.strokes + self.combings + self.swirls <= 4 {
            "Battal"
        } else {
            "Serbest"
        }
    }
}

/// Whether a point lies inside the most recent ink covering it.
fn lands_inside_ink(tray: &Tray, x: f64, y: f64) -> bool {
    tray.inks.iter().rev().take(400).any(|ink| {
        let pts = &ink.outline;
        let mut inside = false;
        let mut j = pts.len() - 1;
        for i in 0..pts.len() {
            let (a, b) = (pts[i], pts[j]);
            if (a.y > y) != (b.y > y) && x < (b.x - a.x) * (y - a.y) / (b.y - a.y) + a.x {
                inside = !inside;
            }
            j = i;
        }
        inside
    })
}

/// A plan of ops the tray carries out by itself when nobody's at it: a
/// composition in one traditional pattern, a movement at a time.
pub struct Composition {
    pub pattern: &'static str,
    pub ops: std::collections::VecDeque<Op>,
}

const STROKE: f64 = 80.0;

/// Splits a long straight stroke into pieces short enough to be one op each.
fn stroke(ops: &mut std::collections::VecDeque<Op>, from: [f64; 2], to: [f64; 2], tines: u8, spacing: f64, reach: f64) {
    let (dx, dy) = (to[0] - from[0], to[1] - from[1]);
    let length = (dx * dx + dy * dy).sqrt();
    let pieces = (length / STROKE).ceil().max(1.0) as usize;
    for k in 0..pieces {
        let (t0, t1) = (k as f64 / pieces as f64, (k + 1) as f64 / pieces as f64);
        ops.push_back(Op::Rake {
            from: [from[0] + dx * t0, from[1] + dy * t0],
            to: [from[0] + dx * t1, from[1] + dy * t1],
            tines,
            spacing,
            reach,
        });
    }
}

impl Composition {
    /// Picks something to make, given what's already on the tray.
    pub fn plan(tray: &Tray, random: &mut Random) -> Composition {
        let mut ops = std::collections::VecDeque::new();
        let palette: Vec<u8> = {
            let mut colours: Vec<u8> = (0..PIGMENTS.len() as u8).filter(|p| *p != tray.ground).collect();
            for i in (1..colours.len()).rev() {
                colours.swap(i, random.below(i + 1));
            }
            colours.truncate(3 + random.below(2));
            colours
        };

        if tray.inks.len() < 40 {
            // Start by covering the bath: many drops, big and small, so each
            // new one packs the ones around it into cells. A few are set
            // inside earlier drops.
            let count = 150 + random.below(70);
            let mut placed: Vec<(f64, f64)> = Vec::new();
            for i in 0..count {
                let (x, y) = if i > 20 && random.unit() < 0.12 {
                    let (px, py) = *random.pick(&placed);
                    (px + random.range(-4.0, 4.0), py + random.range(-4.0, 4.0))
                } else {
                    (random.range(20.0, WIDTH - 20.0), random.range(20.0, HEIGHT - 20.0))
                };
                placed.push((x, y));
                let radius = if random.unit() < 0.3 { random.range(6.0, 14.0) } else { random.range(30.0, 76.0) };
                ops.push_back(Op::Drop { x, y, radius, pigment: *random.pick(&palette) });
            }
            return Composition { pattern: "Battal", ops };
        }

        match random.below(3) {
            0 => {
                // Gel-git: back and forth across the tray.
                let rows = 10 + random.below(5);
                let gap = HEIGHT / rows as f64;
                for r in 0..rows {
                    let y = gap * (r as f64 + 0.5);
                    let (a, b) = if r % 2 == 0 { (20.0, WIDTH - 20.0) } else { (WIDTH - 20.0, 20.0) };
                    stroke(&mut ops, [a, y], [b, y], 1, 20.0, (gap * 0.3).clamp(2.0, 40.0));
                }
                Composition { pattern: "Gel-git", ops }
            }
            1 => {
                // Taraklı: gel-git first if the bath is still uncombed, then a
                // fine comb drawn across the rows.
                if tray.points() < 20_000 {
                    let rows = 10;
                    let gap = HEIGHT / rows as f64;
                    for r in 0..rows {
                        let y = gap * (r as f64 + 0.5);
                        let (a, b) = if r % 2 == 0 { (20.0, WIDTH - 20.0) } else { (WIDTH - 20.0, 20.0) };
                        stroke(&mut ops, [a, y], [b, y], 1, 20.0, gap * 0.3);
                    }
                }
                let tines = 12;
                let spacing = 28.0;
                let width = spacing * (tines as f64 - 1.0);
                // Enough passes to cover the tray edge to edge, spread evenly.
                let span = WIDTH - 40.0;
                let passes = (span / (width + spacing)).ceil() as usize;
                for p in 0..passes {
                    let x = 20.0 + (p as f64 + 0.5) * span / passes as f64;
                    stroke(&mut ops, [x, 20.0], [x, HEIGHT - 20.0], tines, spacing, 7.0);
                }
                Composition { pattern: "Taraklı", ops }
            }
            _ => {
                // Bülbül yuvası: spirals turned across a grid.
                let (cols, rows) = (6 + random.below(3), 4 + random.below(2));
                for r in 0..rows {
                    for c in 0..cols {
                        let x = WIDTH * (c as f64 + 0.5) / cols as f64 + random.range(-20.0, 20.0);
                        let y = HEIGHT * (r as f64 + 0.5) / rows as f64 + random.range(-20.0, 20.0);
                        let direction = if (r + c) % 2 == 0 { 1.0 } else { -1.0 };
                        for _ in 0..4 {
                            ops.push_back(Op::Swirl { x, y, turn: 1.4 * direction, reach: 38.0 });
                        }
                    }
                }
                Composition { pattern: "Bülbül yuvası", ops }
            }
        }
    }
}

/// The ground for the next tray, different from the last one.
pub fn next_ground(previous: u8, random: &mut Random) -> u8 {
    loop {
        let ground = *random.pick(&GROUNDS);
        if ground != previous {
            return ground;
        }
    }
}
