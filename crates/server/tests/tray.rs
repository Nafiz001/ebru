//! The whole server over real sockets: ops in, ops out, snapshots, prints.

use std::time::Duration;

use ebru_server::{AppState, Config, Workshop, http, store::Store};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;

async fn start() -> (String, tempdir::Dir) {
    let dir = tempdir::Dir::new();
    let store = Store::open(&dir.0).unwrap();
    let workshop = Workshop::start(store.clone(), None);
    let state = AppState { workshop, store, config: Config { data: dir.0.clone(), web: None, idle_after: None } };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap().to_string();
    tokio::spawn(async move { axum::serve(listener, http::router(state)).await.unwrap() });
    (address, dir)
}

mod tempdir {
    pub struct Dir(pub std::path::PathBuf);
    impl Dir {
        pub fn new() -> Dir {
            let path = std::env::temp_dir().join(format!("ebru-test-{}-{}", std::process::id(), rand_suffix()));
            std::fs::create_dir_all(&path).unwrap();
            Dir(path)
        }
    }
    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    // Tests run in parallel; a counter keeps their directories apart.
    fn rand_suffix() -> usize {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }
}

/// A plain HTTP/1.1 GET, enough for tests: returns status and body.
async fn get(address: &str, path: &str) -> (u16, Vec<u8>) {
    let mut stream = TcpStream::connect(address).await.unwrap();
    stream.write_all(format!("GET {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await.unwrap();
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let head = String::from_utf8_lossy(&raw[..split]).to_string();
    let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
    assert!(!head.to_ascii_lowercase().contains("transfer-encoding: chunked"), "tests expect sized bodies");
    (status, raw[split + 4..].to_vec())
}

type Socket = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>;

async fn connect(address: &str) -> (Socket, u64) {
    let (mut socket, _) = tokio_tungstenite::connect_async(format!("ws://{address}/ws")).await.unwrap();
    let hello = next_of(&mut socket, "hello").await;
    (socket, hello["you"].as_u64().unwrap())
}

async fn send(socket: &mut Socket, message: Value) {
    socket.send(Message::Text(message.to_string().into())).await.unwrap();
}

/// Waits for the next message of a given type, skipping presence chatter.
async fn next_of(socket: &mut Socket, kind: &str) -> Value {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let message = socket.next().await.unwrap().unwrap();
            if let Message::Text(text) = message {
                let value: Value = serde_json::from_str(&text).unwrap();
                if value["type"] == kind {
                    return value;
                }
            }
        }
    })
    .await
    .unwrap_or_else(|_| panic!("no {kind} message arrived"))
}

fn drop_op(i: usize) -> Value {
    json!({ "type": "op", "op": { "op": "drop", "x": 200.0 + i as f64 * 90.0, "y": 500.0, "radius": 40.0, "pigment": (i % 9) as u8 } })
}

#[tokio::test(flavor = "multi_thread")]
async fn ops_are_numbered_and_reach_everyone() {
    let (address, _dir) = start().await;
    let (mut alice, alice_id) = connect(&address).await;
    let (mut bob, _) = connect(&address).await;

    send(&mut alice, drop_op(0)).await;
    let seen_by_alice = next_of(&mut alice, "op").await;
    let seen_by_bob = next_of(&mut bob, "op").await;
    assert_eq!(seen_by_alice, seen_by_bob);
    assert_eq!(seen_by_bob["seq"], 1);
    assert_eq!(seen_by_bob["by"], alice_id);
    assert_eq!(seen_by_bob["op"]["op"], "drop");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_snapshot_matches_the_checksum_the_server_announces() {
    let (address, _dir) = start().await;
    let (mut socket, _) = connect(&address).await;
    for i in 0..3 {
        send(&mut socket, drop_op(i)).await;
    }
    send(&mut socket, json!({ "type": "op", "op": { "op": "rake", "from": [150.0, 500.0], "to": [380.0, 500.0], "tines": 1, "spacing": 20.0, "reach": 8.0 } })).await;

    let checksum = next_of(&mut socket, "checksum").await;
    assert_eq!(checksum["seq"], 4);

    let (status, bytes) = get(&address, "/api/tray").await;
    assert_eq!(status, 200);
    let tray = marbling::decode(&bytes).unwrap();
    assert_eq!(tray.seq, 4);
    assert_eq!(format!("{:016x}", tray.checksum()), checksum["sum"].as_str().unwrap());
}

#[tokio::test(flavor = "multi_thread")]
async fn bad_ops_are_refused_to_the_person_who_sent_them() {
    let (address, _dir) = start().await;
    let (mut socket, me) = connect(&address).await;
    send(&mut socket, json!({ "type": "op", "op": { "op": "drop", "x": 99999.0, "y": 1.0, "radius": 40.0, "pigment": 0 } })).await;
    let refused = next_of(&mut socket, "refused").await;
    assert_eq!(refused["to"], me);
    assert!(refused["reason"].as_str().unwrap().contains("off the tray"));

    send(&mut socket, json!({ "type": "op", "op": { "op": "skim", "ground": 1 } })).await;
    let refused = next_of(&mut socket, "refused").await;
    assert!(refused["reason"].as_str().unwrap().contains("laying a sheet"));
}

#[tokio::test(flavor = "multi_thread")]
async fn laying_a_sheet_prints_the_tray_and_skims_it() {
    let (address, _dir) = start().await;
    let (mut socket, _) = connect(&address).await;

    send(&mut socket, json!({ "type": "lay" })).await;
    let refused = next_of(&mut socket, "refused").await;
    assert!(refused["reason"].as_str().unwrap().contains("hardly anything"));

    for i in 0..10 {
        send(&mut socket, drop_op(i)).await;
    }
    send(&mut socket, json!({ "type": "lay" })).await;
    let laying = next_of(&mut socket, "laying").await;
    assert_eq!(laying["name"], "Battal");

    let skim = next_of(&mut socket, "op").await;
    let skim = if skim["op"]["op"] == "skim" { skim } else { loop { let op = next_of(&mut socket, "op").await; if op["op"]["op"] == "skim" { break op; } } };
    assert_eq!(skim["seq"], 11);

    let printed = next_of(&mut socket, "printed").await;
    let print = &printed["print"];
    assert_eq!(print["name"], "Battal no. 1");
    assert_eq!(print["people"], 1);
    assert_eq!(print["tally"]["drops"], 10);

    let (status, body) = get(&address, "/api/prints").await;
    assert_eq!(status, 200);
    let list: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(list.as_array().unwrap().len(), 1);

    let id = print["id"].as_i64().unwrap();
    let (status, png) = get(&address, &format!("/api/prints/{id}/image?size=small")).await;
    assert_eq!(status, 200);
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");

    let (status, body) = get(&address, &format!("/api/prints/{id}/recipe")).await;
    assert_eq!(status, 200);
    let recipe: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(recipe["ops"].as_array().unwrap().len(), 10);

    assert_eq!(get(&address, "/api/prints/9999").await.0, 404);
}

#[tokio::test(flavor = "multi_thread")]
async fn everyone_sees_where_everyone_else_is_working() {
    let (address, _dir) = start().await;
    let (mut alice, alice_id) = connect(&address).await;
    let (mut bob, _) = connect(&address).await;
    send(&mut alice, json!({ "type": "cursor", "x": 400.0, "y": 300.0, "tool": "comb", "pigment": 3 })).await;
    let cursor = next_of(&mut bob, "cursor").await;
    assert_eq!(cursor["id"], alice_id);
    assert_eq!(cursor["tool"], "comb");

    drop(alice);
    let left = next_of(&mut bob, "left").await;
    assert_eq!(left["id"], alice_id);
}
