use crate::{HEIGHT, PIGMENTS, WIDTH};

/// Something done to the bath.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "op", rename_all = "snake_case"))]
pub enum Op {
    /// Ink dropped from a brush. It spreads to `radius`, pushing every ink
    /// already on the bath outward to make room.
    Drop { x: f64, y: f64, radius: f64, pigment: u8 },

    /// A stylus (one tine) or a comb (several) drawn in a straight line from
    /// `from` to `to`. Ink near a tine is carried along with it; `reach` is how
    /// far from the tine the pull halves, and `spacing` the gap between tines.
    Rake { from: [f64; 2], to: [f64; 2], tines: u8, spacing: f64, reach: f64 },

    /// A stylus turned in place: ink near the centre rotates by `turn`
    /// radians, less and less further out.
    Swirl { x: f64, y: f64, turn: f64, reach: f64 },

    /// Skim the bath clean and spread a fresh ground.
    Skim { ground: u8 },
}

#[derive(Debug, PartialEq, Eq)]
pub enum OpError {
    OffTheTray,
    NoSuchPigment,
    OutOfRange(&'static str),
    NotANumber,
}

impl std::fmt::Display for OpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpError::OffTheTray => write!(f, "that's off the tray"),
            OpError::NoSuchPigment => write!(f, "there's no such pigment"),
            OpError::OutOfRange(what) => write!(f, "{what} is out of range"),
            OpError::NotANumber => write!(f, "every number must be finite"),
        }
    }
}

impl std::error::Error for OpError {}

const MARGIN: f64 = 40.0;

fn on_tray(x: f64, y: f64) -> bool {
    (-MARGIN..=WIDTH + MARGIN).contains(&x) && (-MARGIN..=HEIGHT + MARGIN).contains(&y)
}

impl Op {
    /// Checks an op from a client before it touches the bath.
    pub fn validate(&self) -> Result<(), OpError> {
        let finite = |values: &[f64]| values.iter().all(|v| v.is_finite());
        match *self {
            Op::Drop { x, y, radius, pigment } => {
                if !finite(&[x, y, radius]) {
                    return Err(OpError::NotANumber);
                }
                if !on_tray(x, y) {
                    return Err(OpError::OffTheTray);
                }
                if !(4.0..=160.0).contains(&radius) {
                    return Err(OpError::OutOfRange("radius"));
                }
                if pigment as usize >= PIGMENTS.len() {
                    return Err(OpError::NoSuchPigment);
                }
            }
            Op::Rake { from, to, tines, spacing, reach } => {
                if !finite(&[from[0], from[1], to[0], to[1], spacing, reach]) {
                    return Err(OpError::NotANumber);
                }
                if !on_tray(from[0], from[1]) || !on_tray(to[0], to[1]) {
                    return Err(OpError::OffTheTray);
                }
                if !(1..=12).contains(&tines) {
                    return Err(OpError::OutOfRange("tines"));
                }
                if !(8.0..=200.0).contains(&spacing) {
                    return Err(OpError::OutOfRange("spacing"));
                }
                if !(2.0..=40.0).contains(&reach) {
                    return Err(OpError::OutOfRange("reach"));
                }
                let length = ((to[0] - from[0]).powi(2) + (to[1] - from[1]).powi(2)).sqrt();
                if length > 240.0 {
                    return Err(OpError::OutOfRange("stroke length"));
                }
            }
            Op::Swirl { x, y, turn, reach } => {
                if !finite(&[x, y, turn, reach]) {
                    return Err(OpError::NotANumber);
                }
                if !on_tray(x, y) {
                    return Err(OpError::OffTheTray);
                }
                if turn.abs() > 1.5 {
                    return Err(OpError::OutOfRange("turn"));
                }
                if !(10.0..=240.0).contains(&reach) {
                    return Err(OpError::OutOfRange("reach"));
                }
            }
            Op::Skim { ground } => {
                if ground as usize >= PIGMENTS.len() {
                    return Err(OpError::NoSuchPigment);
                }
            }
        }
        Ok(())
    }

    /// How many small steps the op is applied in. Each step is a smaller
    /// version of the same transformation; drops and rakes compose exactly,
    /// so the steps add up to the whole op, and they let clients animate it.
    pub fn substeps(&self) -> u32 {
        match *self {
            Op::Drop { radius, .. } => (radius / 6.0).ceil().clamp(4.0, 16.0) as u32,
            Op::Rake { from, to, .. } => {
                let (dx, dy) = (to[0] - from[0], to[1] - from[1]);
                let length = libm::sqrt(dx * dx + dy * dy);
                (length / 4.0).ceil().clamp(1.0, 60.0) as u32
            }
            Op::Swirl { turn, .. } => (turn.abs() / 0.05).ceil().clamp(2.0, 30.0) as u32,
            Op::Skim { .. } => 1,
        }
    }
}
