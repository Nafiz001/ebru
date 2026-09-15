//! A visitor at the tray, over one WebSocket.
//!
//! ```text
//! browser -> server   {"type":"op","op":{...}}
//!                     {"type":"cursor","x":..,"y":..,"tool":"comb","pigment":2}
//!                     {"type":"lay"}
//! server -> browser   {"type":"hello","you":7,"people":3}
//!                     {"type":"op","seq":..,"op":{...},"by":7 | "tray"}
//!                     {"type":"cursor","id":..,"x":..,"y":..,"tool":..,"pigment":..}
//!                     {"type":"left","id":..}  {"type":"people","count":..}
//!                     {"type":"laying",..}  {"type":"printed",..}
//!                     {"type":"checksum","seq":..,"sum":"0123abcd..."}
//!                     {"type":"refused","to":7,"reason":"..."}
//!                     {"type":"resync"}
//! ```
//!
//! A browser that falls too far behind the broadcast (a backgrounded tab, a
//! slow link) is told to resync: fetch a fresh snapshot and carry on from it.

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use futures_util::{SinkExt, StreamExt};
use marbling::Op;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::error::RecvError;

use crate::AppState;
use crate::workshop::{Command, PersonId};

static NEXT_PERSON: AtomicU32 = AtomicU32::new(1);

const TOOLS: [&str; 4] = ["drop", "stylus", "comb", "swirl"];
const CURSOR_EVERY: Duration = Duration::from_millis(45);

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Incoming {
    Op { op: Op },
    Cursor { x: f64, y: f64, tool: String, pigment: u8 },
    Lay,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Presence<'a> {
    Hello { you: PersonId, people: usize },
    Cursor { id: PersonId, x: f64, y: f64, tool: &'a str, pigment: u8 },
    Left { id: PersonId },
    People { count: usize },
    Resync,
}

fn encode(p: &Presence) -> String {
    serde_json::to_string(p).expect("presence always serializes")
}

pub async fn upgrade(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.max_message_size(16 * 1024).on_upgrade(move |socket| visit(socket, state))
}

async fn visit(socket: WebSocket, state: AppState) {
    let id = NEXT_PERSON.fetch_add(1, Ordering::Relaxed);
    let workshop = state.workshop.clone();
    let mut events = workshop.subscribe();
    let (mut sink, mut stream) = socket.split();

    workshop.arrived();
    let count = workshop.people();
    if sink.send(Message::Text(encode(&Presence::Hello { you: id, people: count }).into())).await.is_err() {
        workshop.left();
        return;
    }
    workshop.broadcast(encode(&Presence::People { count }).into());

    let mut last_cursor = Instant::now() - CURSOR_EVERY;
    loop {
        tokio::select! {
            incoming = stream.next() => {
                let text = match incoming {
                    Some(Ok(Message::Text(text))) => text,
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    Some(Ok(_)) => continue,
                };
                match serde_json::from_str::<Incoming>(&text) {
                    Ok(Incoming::Op { op }) => workshop.send(Command::Op { by: id, op }),
                    Ok(Incoming::Lay) => workshop.send(Command::Lay { by: id }),
                    Ok(Incoming::Cursor { x, y, tool, pigment }) => {
                        let Some(tool) = TOOLS.iter().find(|t| **t == tool) else { continue };
                        if !(x.is_finite() && y.is_finite()) || last_cursor.elapsed() < CURSOR_EVERY {
                            continue;
                        }
                        last_cursor = Instant::now();
                        let (x, y) = (x.clamp(-50.0, 1650.0), y.clamp(-50.0, 1050.0));
                        workshop.broadcast(encode(&Presence::Cursor { id, x, y, tool, pigment }).into());
                    }
                    Err(_) => tracing::debug!(person = id, "ignored a message that didn't parse"),
                }
            }
            event = events.recv() => {
                let text = match event {
                    Ok(text) => text.to_string(),
                    Err(RecvError::Lagged(_)) => encode(&Presence::Resync),
                    Err(RecvError::Closed) => break,
                };
                if sink.send(Message::Text(text.into())).await.is_err() {
                    break;
                }
            }
        }
    }

    workshop.left();
    workshop.broadcast(encode(&Presence::Left { id }).into());
    workshop.broadcast(encode(&Presence::People { count: workshop.people() }).into());
}
