//! The ebru server: one shared marbling tray.
//!
//! A single thread, the [`workshop`], owns the tray. Every op from every
//! visitor goes through it, in order; it applies the op, gives it the next
//! sequence number and announces it to everyone. Browsers run the same
//! marbling code compiled to WebAssembly and apply the same ops in the same
//! order, so every tray in every browser matches the server's to the bit, and
//! a checksum after each quiet spell proves it.
//!
//! When someone lays a sheet, the workshop hands a copy of the tray to a
//! printer thread, which renders it onto paper as a PNG and hangs it in the
//! gallery, and skims the bath for the next pattern.

pub mod http;
pub mod patterns;
pub mod print;
pub mod random;
pub mod store;
pub mod workshop;
pub mod ws;

use std::path::PathBuf;
use std::sync::Arc;

pub use workshop::Workshop;

#[derive(Clone)]
pub struct Config {
    /// Where prints and the database live.
    pub data: PathBuf,
    /// Built web app to serve, if any.
    pub web: Option<PathBuf>,
    /// Quiet seconds before the tray starts marbling on its own. None turns it off.
    pub idle_after: Option<std::time::Duration>,
}

#[derive(Clone)]
pub struct AppState {
    pub workshop: Arc<Workshop>,
    pub store: store::Store,
    pub config: Config,
}
