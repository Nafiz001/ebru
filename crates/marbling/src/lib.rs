//! Mathematical marbling.
//!
//! Marbled paper is made by floating ink on a thickened bath and moving it
//! with a stylus, a comb or a swirl before laying a sheet on top. Aubrey
//! Jaffer's *Mathematical Marbling* (2012) showed that each of those movements
//! is a closed-form transformation of the plane that preserves area, the way
//! the incompressible bath does, so ink never overlaps and never tears.
//!
//! This crate keeps every ink as a polygon and moves its vertices through
//! those transformations. It is written to give bit-identical results on
//! every platform: the server runs it natively, browsers run the same code
//! compiled to WebAssembly, and a checksum proves they agree. That is why all
//! transcendental maths goes through `libm` rather than the platform's own.

mod ops;
mod pigment;
mod snapshot;
mod tray;

pub use ops::{Op, OpError};
pub use pigment::{GROUNDS, PIGMENTS, Pigment, pigment};
pub use snapshot::{SnapshotError, decode, encode};
pub use tray::{FULL, Ink, Point, Tray};

/// The tray, in its own units. Clients scale it to fit.
pub const WIDTH: f64 = 1600.0;
pub const HEIGHT: f64 = 1000.0;
