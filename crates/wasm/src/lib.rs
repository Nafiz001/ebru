//! The marbling tray, exported to the browser as plain WebAssembly functions.
//!
//! No bindings generator: every export takes and returns numbers, and bytes
//! cross the boundary through `alloc`ed buffers. The browser keeps one tray,
//! queues ops as the server announces them, advances a budget of substeps per
//! animation frame, and draws each ink by reading its outline directly out of
//! this module's memory as a `Float64Array` of x, y pairs.

use std::cell::RefCell;

use marbling::{Op, Tray, decode};

thread_local! {
    static TRAY: RefCell<Tray> = RefCell::new(Tray::new(9));
}

fn with<R>(f: impl FnOnce(&mut Tray) -> R) -> R {
    TRAY.with(|tray| f(&mut tray.borrow_mut()))
}

/// Reserves `len` bytes for the caller to write into, e.g. a snapshot.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut buffer = Vec::<u8>::with_capacity(len);
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr
}

/// # Safety
/// `ptr` and `len` must come from a single call to [`alloc`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn release(ptr: *mut u8, len: usize) {
    unsafe { drop(Vec::from_raw_parts(ptr, 0, len)) };
}

/// Replaces the tray with a snapshot. Returns 1 on success, 0 if the bytes
/// aren't a valid snapshot (the tray is left as it was).
///
/// # Safety
/// `ptr` must point to `len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn load(ptr: *const u8, len: usize) -> u32 {
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    match decode(bytes) {
        Ok(tray) => {
            with(|current| *current = tray);
            1
        }
        Err(_) => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn queue_drop(x: f64, y: f64, radius: f64, pigment: u32) {
    with(|t| t.enqueue(Op::Drop { x, y, radius, pigment: pigment as u8 }));
}

#[unsafe(no_mangle)]
pub extern "C" fn queue_rake(fx: f64, fy: f64, tx: f64, ty: f64, tines: u32, spacing: f64, reach: f64) {
    with(|t| t.enqueue(Op::Rake { from: [fx, fy], to: [tx, ty], tines: tines as u8, spacing, reach }));
}

#[unsafe(no_mangle)]
pub extern "C" fn queue_swirl(x: f64, y: f64, turn: f64, reach: f64) {
    with(|t| t.enqueue(Op::Swirl { x, y, turn, reach }));
}

#[unsafe(no_mangle)]
pub extern "C" fn queue_skim(ground: u32) {
    with(|t| t.enqueue(Op::Skim { ground: ground as u8 }));
}

/// Applies up to `budget` substeps; returns how many are still queued.
#[unsafe(no_mangle)]
pub extern "C" fn advance(budget: u32) -> u32 {
    with(|t| t.advance(budget))
}

/// Sequence number of the last op fully applied. Exact up to 2^53, which is
/// a great many strokes.
#[unsafe(no_mangle)]
pub extern "C" fn seq() -> f64 {
    with(|t| t.seq as f64)
}

#[unsafe(no_mangle)]
pub extern "C" fn settled() -> u32 {
    with(|t| t.is_settled() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn checksum_high() -> u32 {
    with(|t| (t.checksum() >> 32) as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn checksum_low() -> u32 {
    with(|t| t.checksum() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn ground() -> u32 {
    with(|t| t.ground as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn points() -> u32 {
    with(|t| t.points() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn full() -> u32 {
    with(|t| t.is_full() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn ink_count() -> u32 {
    with(|t| t.inks.len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn ink_pigment(index: u32) -> u32 {
    with(|t| t.inks.get(index as usize).map_or(0, |ink| ink.pigment as u32))
}

#[unsafe(no_mangle)]
pub extern "C" fn ink_length(index: u32) -> u32 {
    with(|t| t.inks.get(index as usize).map_or(0, |ink| ink.outline.len() as u32))
}

/// Where an ink's outline starts in memory: `ink_length` pairs of f64 x, y.
/// Valid only until the tray next changes.
#[unsafe(no_mangle)]
pub extern "C" fn ink_outline(index: u32) -> *const f64 {
    with(|t| t.inks.get(index as usize).map_or(std::ptr::null(), |ink| ink.outline.as_ptr() as *const f64))
}
