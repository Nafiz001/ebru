//! The whole tray as bytes, for a newcomer to start from.
//!
//! ```text
//! "EBRU" version:u8 seq:u64 ground:u8 inks:u32
//!   { pigment:u8 points:u32 { x:f64 y:f64 }* }*
//! ```
//!
//! Little-endian throughout, and coordinates are kept as full f64 so a tray
//! restored from a snapshot continues bit-for-bit like the original.

use crate::tray::{Ink, Point, Tray};

const MAGIC: &[u8; 4] = b"EBRU";
const VERSION: u8 = 1;

#[derive(Debug, PartialEq, Eq)]
pub enum SnapshotError {
    NotASnapshot,
    UnsupportedVersion(u8),
    Truncated,
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnapshotError::NotASnapshot => write!(f, "not an ebru snapshot"),
            SnapshotError::UnsupportedVersion(v) => write!(f, "snapshot version {v} isn't supported"),
            SnapshotError::Truncated => write!(f, "snapshot is truncated"),
        }
    }
}

impl std::error::Error for SnapshotError {}

pub fn encode(tray: &Tray) -> Vec<u8> {
    let mut out = Vec::with_capacity(18 + tray.points() * 16 + tray.inks.len() * 5);
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.extend_from_slice(&tray.seq.to_le_bytes());
    out.push(tray.ground);
    out.extend_from_slice(&(tray.inks.len() as u32).to_le_bytes());
    for ink in &tray.inks {
        out.push(ink.pigment);
        out.extend_from_slice(&(ink.outline.len() as u32).to_le_bytes());
        for p in &ink.outline {
            out.extend_from_slice(&p.x.to_le_bytes());
            out.extend_from_slice(&p.y.to_le_bytes());
        }
    }
    out
}

struct Reader<'a> {
    bytes: &'a [u8],
}

impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], SnapshotError> {
        if self.bytes.len() < N {
            return Err(SnapshotError::Truncated);
        }
        let (head, rest) = self.bytes.split_at(N);
        self.bytes = rest;
        Ok(head.try_into().expect("split at N"))
    }
}

pub fn decode(bytes: &[u8]) -> Result<Tray, SnapshotError> {
    let mut r = Reader { bytes };
    if &r.take::<4>().map_err(|_| SnapshotError::NotASnapshot)? != MAGIC {
        return Err(SnapshotError::NotASnapshot);
    }
    let [version] = r.take::<1>()?;
    if version != VERSION {
        return Err(SnapshotError::UnsupportedVersion(version));
    }
    let seq = u64::from_le_bytes(r.take()?);
    let [ground] = r.take::<1>()?;
    let count = u32::from_le_bytes(r.take()?) as usize;
    let mut inks = Vec::with_capacity(count.min(100_000));
    for _ in 0..count {
        let [pigment] = r.take::<1>()?;
        let points = u32::from_le_bytes(r.take()?) as usize;
        if r.bytes.len() < points * 16 {
            return Err(SnapshotError::Truncated);
        }
        let mut outline = Vec::with_capacity(points);
        for _ in 0..points {
            let x = f64::from_le_bytes(r.take()?);
            let y = f64::from_le_bytes(r.take()?);
            outline.push(Point { x, y });
        }
        inks.push(Ink::new(pigment, outline));
    }
    Ok(Tray::from_parts(ground, seq, inks))
}
