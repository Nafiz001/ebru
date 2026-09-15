use std::collections::VecDeque;

use crate::{HEIGHT, Op, WIDTH};

/// A point on the tray. `repr(C)` so a run of points is a plain run of f64s,
/// which the browser reads straight out of WebAssembly memory.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// One drop of ink: its pigment and the outline it has been pulled into.
#[derive(Clone, Debug, PartialEq)]
pub struct Ink {
    pub pigment: u8,
    pub outline: Vec<Point>,
    /// Bounding box, kept current so movements can skip inks they can't reach.
    min: Point,
    max: Point,
    /// Bounding boxes of successive runs of `CHUNK` points. Combed ink becomes
    /// long thin filaments whose overall box covers the tray; these let a
    /// stroke skip the parts of a filament it can't reach.
    chunks: Vec<(Point, Point)>,
    /// Moved since it was last tidied.
    moved: bool,
}

const CHUNK: usize = 64;

impl Ink {
    pub fn new(pigment: u8, outline: Vec<Point>) -> Self {
        let origin = Point { x: 0.0, y: 0.0 };
        let mut ink = Ink { pigment, outline, min: origin, max: origin, chunks: Vec::new(), moved: false };
        ink.refresh_bounds();
        ink
    }

    fn refresh_bounds(&mut self) {
        let mut min = Point { x: f64::INFINITY, y: f64::INFINITY };
        let mut max = Point { x: f64::NEG_INFINITY, y: f64::NEG_INFINITY };
        self.chunks.clear();
        for run in self.outline.chunks(CHUNK) {
            let mut lo = Point { x: f64::INFINITY, y: f64::INFINITY };
            let mut hi = Point { x: f64::NEG_INFINITY, y: f64::NEG_INFINITY };
            for p in run {
                lo.x = lo.x.min(p.x);
                lo.y = lo.y.min(p.y);
                hi.x = hi.x.max(p.x);
                hi.y = hi.y.max(p.y);
            }
            min.x = min.x.min(lo.x);
            min.y = min.y.min(lo.y);
            max.x = max.x.max(hi.x);
            max.y = max.y.max(hi.y);
            self.chunks.push((lo, hi));
        }
        self.min = min;
        self.max = max;
    }

    /// The runs of outline points whose boxes touch the box `lo`–`hi`.
    fn runs_near(&mut self, lo: Point, hi: Point) -> impl Iterator<Item = &mut [Point]> {
        let chunks = &self.chunks;
        self.outline
            .chunks_mut(CHUNK)
            .zip(chunks.iter())
            .filter(move |(_, (a, b))| !(b.x < lo.x || a.x > hi.x || b.y < lo.y || a.y > hi.y))
            .map(|(run, _)| run)
    }

    fn overlaps(&self, lo: Point, hi: Point) -> bool {
        !(self.max.x < lo.x || self.min.x > hi.x || self.max.y < lo.y || self.min.y > hi.y)
    }

    /// Signed area by the shoelace formula; positive for anticlockwise outlines.
    pub fn area(&self) -> f64 {
        let n = self.outline.len();
        let mut twice = 0.0;
        for i in 0..n {
            let a = self.outline[i];
            let b = self.outline[(i + 1) % n];
            twice += a.x * b.y - b.x * a.y;
        }
        twice / 2.0
    }
}

/// Longest an outline edge may get before it's split, and shortest before
/// neighbouring points are merged, in tray units.
const MAX_EDGE: f64 = 4.5;
const MIN_EDGE: f64 = 0.6;
/// Inks pushed this far past the edge of the tray can never be seen again.
const CULL_MARGIN: f64 = 600.0;
/// Above this many outline points the bath is too busy to keep working.
pub const FULL: usize = 450_000;

#[derive(Clone, Debug)]
struct Pending {
    op: Op,
    step: u32,
    steps: u32,
}

/// The bath and everything floating on it.
#[derive(Clone, Debug)]
pub struct Tray {
    pub ground: u8,
    pub inks: Vec<Ink>,
    /// The sequence number of the last op applied.
    pub seq: u64,
    pending: VecDeque<Pending>,
    scratch: Vec<Point>,
}

impl Tray {
    pub fn new(ground: u8) -> Self {
        Tray { ground, inks: Vec::new(), seq: 0, pending: VecDeque::new(), scratch: Vec::new() }
    }

    pub(crate) fn from_parts(ground: u8, seq: u64, inks: Vec<Ink>) -> Self {
        Tray { ground, inks, seq, pending: VecDeque::new(), scratch: Vec::new() }
    }

    /// Applies an op completely, now.
    pub fn apply(&mut self, op: &Op) {
        self.enqueue(op.clone());
        while self.advance(u32::MAX) > 0 {}
    }

    /// Queues an op to be applied a step at a time by [`Tray::advance`].
    /// Ops always finish in the order they were queued.
    pub fn enqueue(&mut self, op: Op) {
        let steps = op.substeps();
        self.pending.push_back(Pending { op, step: 0, steps });
    }

    /// Applies up to `budget` substeps of queued ops; returns how many remain.
    pub fn advance(&mut self, budget: u32) -> u32 {
        let mut done = 0;
        while done < budget {
            let Some(front) = self.pending.front_mut() else { break };
            let (op, step, steps) = (front.op.clone(), front.step, front.steps);
            front.step += 1;
            if front.step == front.steps {
                self.pending.pop_front();
                self.seq += 1;
            }
            self.substep(&op, step, steps);
            done += 1;
        }
        self.pending.iter().map(|p| p.steps - p.step).sum()
    }

    pub fn is_settled(&self) -> bool {
        self.pending.is_empty()
    }

    pub fn points(&self) -> usize {
        self.inks.iter().map(|i| i.outline.len()).sum()
    }

    pub fn is_full(&self) -> bool {
        self.points() > FULL
    }

    /// FNV-1a over every bit of the tray's state. Two trays that have seen the
    /// same ops, on any platform, have the same checksum.
    pub fn checksum(&self) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        let mut eat = |bytes: &[u8]| {
            for b in bytes {
                hash ^= *b as u64;
                hash = hash.wrapping_mul(0x100000001b3);
            }
        };
        eat(&[self.ground]);
        eat(&self.seq.to_le_bytes());
        for ink in &self.inks {
            eat(&[ink.pigment]);
            eat(&(ink.outline.len() as u64).to_le_bytes());
            for p in &ink.outline {
                eat(&p.x.to_bits().to_le_bytes());
                eat(&p.y.to_bits().to_le_bytes());
            }
        }
        hash
    }

    fn substep(&mut self, op: &Op, step: u32, steps: u32) {
        let n = steps as f64;
        match *op {
            Op::Drop { x, y, radius, pigment } => {
                // Spreading to r² in n steps of r²/n is exactly one spread to r²:
                // |p'-c|² = |p-c|² + r², and squared radii add.
                let r2 = radius * radius / n;
                let c = Point { x, y };
                for ink in &mut self.inks {
                    for p in &mut ink.outline {
                        push_out(p, c, r2);
                    }
                    ink.moved = true;
                }
                if step == 0 {
                    self.inks.push(Ink::new(pigment, circle(c, libm::sqrt(r2))));
                }
            }
            Op::Rake { from, to, tines, spacing, reach } => {
                let (dx, dy) = (to[0] - from[0], to[1] - from[1]);
                let length = libm::sqrt(dx * dx + dy * dy);
                if length < 1e-9 {
                    return;
                }
                let flow = Rake::new(from, [dx / length, dy / length], length, tines, spacing, reach);
                let shift = length / n;
                let (lo, hi) = flow.bounds();
                for ink in &mut self.inks {
                    if !ink.overlaps(lo, hi) {
                        continue;
                    }
                    let mut moved = false;
                    for run in ink.runs_near(lo, hi) {
                        for p in run {
                            // Midpoint integration: the flow bends, and a straight
                            // Euler step along it slowly gains or loses area.
                            if let Some((vx, vy)) = flow.velocity(*p) {
                                let mid = Point { x: p.x + vx * shift * 0.5, y: p.y + vy * shift * 0.5 };
                                let (wx, wy) = flow.velocity(mid).unwrap_or((vx, vy));
                                p.x += wx * shift;
                                p.y += wy * shift;
                                moved = true;
                            }
                        }
                    }
                    ink.moved |= moved;
                }
            }
            Op::Swirl { x, y, turn, reach } => {
                // Rotation keeps distance from the centre, so the angles of n
                // steps add up exactly.
                let angle = turn / n;
                let limit = reach * 10.0;
                let (lo, hi) = (Point { x: x - limit, y: y - limit }, Point { x: x + limit, y: y + limit });
                for ink in &mut self.inks {
                    if !ink.overlaps(lo, hi) {
                        continue;
                    }
                    let mut moved = false;
                    for run in ink.runs_near(lo, hi) {
                        for p in run {
                            let (rx, ry) = (p.x - x, p.y - y);
                            let d = libm::sqrt(rx * rx + ry * ry);
                            if d > limit {
                                continue;
                            }
                            let theta = angle * libm::exp(-d / reach);
                            let (s, c) = (libm::sin(theta), libm::cos(theta));
                            p.x = x + rx * c - ry * s;
                            p.y = y + rx * s + ry * c;
                            moved = true;
                        }
                    }
                    ink.moved |= moved;
                }
            }
            Op::Skim { ground } => {
                self.inks.clear();
                self.ground = ground;
            }
        }
        self.tidy();
    }

    /// Keeps outlines smooth and bounded: split long edges, merge crowded
    /// points, and forget inks pushed irretrievably off the tray.
    fn tidy(&mut self) {
        let scratch = &mut self.scratch;
        for ink in self.inks.iter_mut().filter(|ink| ink.moved) {
            ink.moved = false;
            let n = ink.outline.len();
            if n < 3 {
                continue;
            }
            // Most small steps leave every edge within bounds; check before rebuilding.
            let needs_work = (0..n).any(|i| {
                let (a, b) = (ink.outline[i], ink.outline[(i + 1) % n]);
                let (ex, ey) = (b.x - a.x, b.y - a.y);
                let length2 = ex * ex + ey * ey;
                length2 > MAX_EDGE * MAX_EDGE || length2 < MIN_EDGE * MIN_EDGE
            });
            if !needs_work {
                ink.refresh_bounds();
                continue;
            }
            scratch.clear();
            for i in 0..n {
                let a = ink.outline[i];
                let b = ink.outline[(i + 1) % n];
                // A point crowding the one before it adds nothing to the shape.
                let crowded = scratch.last().is_some_and(|last: &Point| {
                    let (ex, ey) = (a.x - last.x, a.y - last.y);
                    ex * ex + ey * ey < MIN_EDGE * MIN_EDGE
                });
                if !crowded {
                    scratch.push(a);
                }
                let (ex, ey) = (b.x - a.x, b.y - a.y);
                let length = libm::sqrt(ex * ex + ey * ey);
                if length > MAX_EDGE {
                    let pieces = libm::ceil(length / MAX_EDGE) as usize;
                    for k in 1..pieces {
                        let t = k as f64 / pieces as f64;
                        scratch.push(Point { x: a.x + ex * t, y: a.y + ey * t });
                    }
                }
            }
            if scratch.len() >= 3 {
                std::mem::swap(&mut ink.outline, scratch);
            }
            ink.refresh_bounds();
        }
        self.inks.retain(|ink| {
            ink.max.x > -CULL_MARGIN
                && ink.min.x < WIDTH + CULL_MARGIN
                && ink.max.y > -CULL_MARGIN
                && ink.min.y < HEIGHT + CULL_MARGIN
        });
    }
}

/// The flow of the bath around tines drawn through it.
///
/// Jaffer's tine line is infinite: it moves every point parallel to the line
/// by an amount that depends only on distance from it, which can't compress
/// anything. A stroke that starts and stops can't work that way, or ink would
/// pile up at its ends. So the flow here comes from a stream function
///
/// ```text
/// ψ(across, along) = λ · H(across) · G(along)
/// v = (∂ψ/∂across, −∂ψ/∂along)
/// ```
///
/// which is divergence-free by construction: whatever the tine pushes forward,
/// the bath returns around it. `H` pulls hard near a tine and dies away (with a
/// gentle counter-current) a few reaches off; `G` is one along the stroke and
/// eases to zero past either end. Summing ψ over tines makes a comb.
/// How fast ink right at a tine moves, as a fraction of the tine's speed.
const SLIP: f64 = 0.5;

struct Rake {
    origin: [f64; 2],
    along: [f64; 2],
    across: [f64; 2],
    length: f64,
    offsets: Vec<f64>,
    reach: f64,
    ease: f64,
    fade: f64,
}

impl Rake {
    fn new(origin: [f64; 2], along: [f64; 2], length: f64, tines: u8, spacing: f64, reach: f64) -> Self {
        let half = (tines as f64 - 1.0) / 2.0;
        Rake {
            origin,
            along,
            across: [-along[1], along[0]],
            length,
            offsets: (0..tines).map(|k| (k as f64 - half) * spacing).collect(),
            reach,
            ease: reach * 1.5,
            fade: reach * 6.0,
        }
    }

    fn bounds(&self) -> (Point, Point) {
        let pad = self.offsets.iter().fold(0.0_f64, |m, o| m.max(o.abs())) + self.reach * 14.0 + self.ease * 4.0;
        let end = [self.origin[0] + self.along[0] * self.length, self.origin[1] + self.along[1] * self.length];
        (
            Point { x: self.origin[0].min(end[0]) - pad, y: self.origin[1].min(end[1]) - pad },
            Point { x: self.origin[0].max(end[0]) + pad, y: self.origin[1].max(end[1]) + pad },
        )
    }

    /// Velocity at a point for a stroke of unit length, or None if it's out of reach.
    fn velocity(&self, p: Point) -> Option<(f64, f64)> {
        let (rx, ry) = (p.x - self.origin[0], p.y - self.origin[1]);
        let t = rx * self.along[0] + ry * self.along[1];
        let a = rx * self.across[0] + ry * self.across[1];
        // Past these the flow is below a thousandth of the stroke; call it still.
        if t < -self.ease * 4.0 || t > self.length + self.ease * 4.0 {
            return None;
        }

        // G and dG/dt: a soft window over the stroke.
        let (u0, u1) = (t / self.ease, (t - self.length) / self.ease);
        let (t0, t1) = (libm::tanh(u0), libm::tanh(u1));
        let g = 0.5 * (t0 - t1);
        let dg = 0.5 / self.ease * ((1.0 - t0 * t0) - (1.0 - t1 * t1));

        let mut v_along = 0.0;
        let mut v_across = 0.0;
        let mut touched = false;
        for offset in &self.offsets {
            let d = a - offset;
            if d.abs() > self.reach * 14.0 {
                continue;
            }
            touched = true;
            // H(d) = tanh(d/λ) · exp(-(d/F)²), and λ·H'(d).
            let th = libm::tanh(d / self.reach);
            let envelope = libm::exp(-(d * d) / (self.fade * self.fade));
            let h = th * envelope;
            let dh = ((1.0 - th * th) - self.reach * th * 2.0 * d / (self.fade * self.fade)) * envelope;
            v_along += dh * g;
            v_across -= self.reach * h * dg;
        }
        if !touched {
            return None;
        }
        // Overlapping tines don't carry ink faster than the comb moves, and the
        // bath slips past a tine rather than being carried all the way with it.
        // Scaling ψ by a constant keeps the flow incompressible.
        let pull = v_along.abs().max(1.0) / SLIP;
        let (v_along, v_across) = (v_along / pull, v_across / pull);
        Some((
            self.along[0] * v_along + self.across[0] * v_across,
            self.along[1] * v_along + self.across[1] * v_across,
        ))
    }
}

fn push_out(p: &mut Point, c: Point, r2: f64) {
    let (dx, dy) = (p.x - c.x, p.y - c.y);
    let d2 = dx * dx + dy * dy;
    if d2 < 1e-12 {
        return;
    }
    let scale = libm::sqrt(1.0 + r2 / d2);
    p.x = c.x + dx * scale;
    p.y = c.y + dy * scale;
}

fn circle(c: Point, r: f64) -> Vec<Point> {
    let n = 24;
    (0..n)
        .map(|i| {
            let a = core::f64::consts::TAU * i as f64 / n as f64;
            Point { x: c.x + r * libm::cos(a), y: c.y + r * libm::sin(a) }
        })
        .collect()
}
