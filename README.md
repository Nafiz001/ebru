# ebru

A shared marbling tray that anyone can work in at once. Drop ink onto the bath,
draw it out with a stylus, comb it, turn it, then lay a sheet of paper on it and
hang the print in the gallery. Rust on the back, and Rust again in the browser
as WebAssembly, under a Vue front end.

Ebru is Turkish paper marbling: pigment floated on thickened water, pushed into
patterns, and lifted off onto paper.

- Every drop pushes all the ink already on the bath outward, the way oil paint
  spreads on water. Rings inside rings build up without any ink ever mixing.
- A stylus or a comb drags the ink with it and leaves the rest of the bath
  alone. Drawn back and forth, rows of drops pull into chevrons; combed across,
  they break into feathers.
- Everyone at the tray sees everyone else's hands move and the ink respond, live.
- Laying a sheet prints the tray at 2400 pixels on generated laid paper, names
  the print after its traditional pattern ("Taraklı no. 3"), and skims the bath
  clean for the next one.
- Every print keeps its recipe, the exact list of movements that made it. A
  print's page can replay the tray being made, stroke by stroke.
- Left alone with people watching, the tray marbles by itself: a battal ground,
  then a gel-git, a combed taraklı or a bülbül yuvası of spirals.

```console
$ make setup
$ make dev
server  http://localhost:8740
web     http://localhost:5174
```

Nothing is a bitmap until the paper goes down. Each drop of ink is a polygon,
and every movement moves every point on every polygon exactly, by the
mathematics of the fluid, not a simulation of it.

## The marbling

`crates/marbling` is the tray: plain Rust, no dependencies but `libm`. The same
crate runs on the server and, compiled to WebAssembly, in every browser.

### Movements are exact maps of the plane

The maths follows Aubrey Jaffer's work on mathematical marbling. Ink floats and
doesn't mix, so the bath can be described entirely by where each ink boundary
goes.

- **A drop** of radius *r* at *c* sends every point *p* to
  *c* + (*p* − *c*)·√(1 + *r*²/|*p* − *c*|²). Area is conserved exactly: the
  ink that was there is pushed into a thinner ring around the new drop. The
  tray applies large drops in several substeps (the maps compose by adding
  *r*²), so an outline has a chance to grow new points as it stretches.
- **A swirl** rotates each point about the centre by an angle that falls off
  with distance, *turn*·exp(−*d*/*reach*). Each ring turns rigidly, so area is
  conserved, and the substeps compose exactly.
- **A stroke or comb** is the hard one. A naive "push points along the stroke"
  map piles ink up and tears it apart. Instead the stroke defines a stream
  function ψ = λ·H(across)·G(along), and ink moves with the velocity
  (∂ψ/∂across, −∂ψ/∂along). A flow derived from a stream function is
  divergence-free, so it cannot create or destroy area: drawn chevrons keep
  exactly as much ink as the drops they came from. H = tanh(*d*/λ)·exp(−(*d*/6λ)²)
  gives the sharp shear right at the tine and the gentle drag further out; G
  eases in and out at the ends of the stroke. The tray integrates it with a
  midpoint (RK2) step per substep. The tests check a combed drop's area stays
  within 2%.

### Outlines stay honest

As ink stretches, the space between outline points grows. After every
substep the moved inks are tidied: edges longer than 4.5 tray units are split,
and runs shorter than 0.6 are merged. A drop starts as a 24-point circle; a
thin filament after a few combings can hold thousands of points.

Keeping that cheap on a busy tray, with hundreds of inks and hundreds of
thousands of points:

- Every ink keeps its bounding box, and a box for each run of 64 points, so a
  stroke only touches the parts of a filament it can reach.
- Only inks that actually moved are tidied.
- Inks pushed 600 units past the edge are dropped for good.
- Past 450,000 points the tray says it is full and asks for a sheet to be laid.

`cargo run -p marbling --release --example bench` times it: 120 drops, then
four passes of gel-git strokes through them, at about a millisecond per
substep.

### Bit for bit, everywhere

A browser doesn't receive the tray's ink; it receives the movements, numbered,
and runs them on its own copy of the tray. That only works if every copy lands
on the same bits.

- Every square root, tanh and exponential goes through `libm`, the same
  software implementation on x86-64 and wasm32, never the platform's maths
  library.
- Ops travel as JSON with round-trip-exact floats.
- Once the tray has been still for a moment, the server broadcasts a 64-bit
  FNV-1a checksum over every coordinate of every ink. The browser compares it
  to its own. On a mismatch, or a gap in the sequence numbers, it quietly
  fetches a fresh binary snapshot and carries on.

`crates/marbling/tests/fixtures/recipe.json` is a real print, 648 movements made
by hand. A test replays it to its recorded checksum, and
`web/scripts/determinism.mjs` replays it natively and in WebAssembly side by
side and compares the checksums after every movement. CI runs both.

## How the server works

`crates/server` is axum on tokio, one binary called `ebru`.

### One workshop, one thread

The tray is owned by a single plain thread, the workshop. Nothing else touches
it, so there are no locks around the ink.

- WebSocket handlers send it commands over a channel: a movement, a request for
  a snapshot, a sheet to lay.
- It validates each movement (inside the tray, a sensible size, a stroke no
  longer than 240 units), numbers it, applies it and broadcasts it, already
  serialized once, to every connection.
- Hands are rate limited with a token bucket per person: bursts of 200
  movements, refilling at 60 a second, which a person can't outdraw but a
  script can't flood. Refusals go back only to the hand that was refused.
- If a connection falls behind the broadcast, it's told to resync rather than
  sent a tray with holes in it.

Cursors don't go through the workshop at all: they're throttled to one every
45 ms per person and relayed with the tool each person is holding, so you can
see someone's comb hovering before they drag it.

### Laying a sheet

A sheet needs at least eight movements on the bath. The workshop takes a copy
of the tray and skims the bath straight away onto a new ground colour, so
people can keep working, while another thread prints the copy:

- `print.rs` renders every ink as a filled path with `tiny-skia`, at 2400
  pixels and at 640 for the gallery. Inks are filled in the order they were
  dropped; since a new drop pushes old ink aside instead of covering it, nothing
  overlaps and that order is exact. Value noise then darkens the sheet with
  fibre running one way and blotches where the paper took up more ink.
- `patterns.rs` names it. The workshop has been keeping a tally of drops,
  drops landed inside other ink, stylus strokes, combings and swirls:

  | | |
  | --- | --- |
  | **Bülbül yuvası** | "nightingale's nest": eight or more swirls |
  | **Taraklı** | combed: six or more combings |
  | **Hatip** | drops within drops, drawn through with a stylus, after Hatip Mehmed Efendi |
  | **Gel-git** | "come and go": a dozen strokes back and forth |
  | **Battal** | drops alone, the oldest ebru |
  | **Serbest** | "free": anything else |

- `store.rs` numbers it within its pattern, and saves the PNGs, the recipe, and
  a row in SQLite with who was at the tray.

### The tray marbling by itself

When nobody has touched the tray for 20 seconds but someone is watching, the
workshop starts composing on its own, one movement at a time, at an unhurried
pace:

1. If the bath is bare, a battal: 150–220 drops in three or four pigments, big
   and small, packing each other into cells, a few set inside earlier drops.
2. Ten seconds' rest to let people look.
3. A gel-git of 10–14 rows back and forth; or a taraklı, a 12-tine comb pulled
   in evenly spaced passes across the whole bath; or a bülbül yuvası, a grid of
   spirals turning alternately left and right.

Any hand on the tray stops it at once. When the tray has made a pattern entirely
by itself, a battal and a movement through it, it lays its own sheet, at most
once every ten minutes, and begins again on the new ground. `cargo run -p ebru-server --release --example compose` renders its
compositions to PNG without a server.

### API

| | |
| --- | --- |
| `GET /api/tray` | the whole tray as a binary snapshot |
| `GET /api/pigments` | the ten pigments, and which can be a ground |
| `GET /api/prints?before=&limit=` | the gallery, newest first |
| `GET /api/prints/{id}` | one print: name, pattern, when, how many people, tally |
| `GET /api/prints/{id}/image?size=small` | the PNG |
| `GET /api/prints/{id}/recipe` | the ground and every movement |
| `GET /health` | |
| `WS /ws` | send `op`, `cursor`, `lay`; receive `op`, `checksum`, `laying`, `printed`, `refused`, `hello`, `cursor`, `people`, `left`, `resync` |

The snapshot is `EBRU`, a version byte, the sequence number, the ground, the
number of inks, and then each ink as its pigment, a point count and little-endian f64 pairs.

## How the browser works

The tray runs in a Web Worker, as a 60 KB WebAssembly module exported without
wasm-bindgen: a handful of `extern "C"` functions over a thread-local tray.

- The worker loads the snapshot, then applies movements as they arrive, spending
  12–30 ms per tick so large movements play out over a few frames instead of
  freezing the page. The ink flows visibly, as it does on water.
- Each frame, it reads the outlines straight out of WebAssembly memory, packs
  them into one `Float32Array` and transfers it to the page with no copy.
- `TrayCanvas.vue` fills each ink as a path on a 2D canvas, with a faint darker
  rim like the edge ox gall leaves where one ink meets another. A second canvas holds
  the overlay: stylus trails, drop rings, and other people's hands.

**Tools.** A brush drops ink with a click, or flicks a trail of drops with a
drag. The stylus and the comb (nine tines) turn a drag into strokes, coalesced
so a quick hand sends a movement every 40 ms rather than one per pointer event.
Hold the swirl to keep turning; hold Shift to turn the other way. Each tool has
three sizes.

**The sheet.** Laying a sheet lowers a sheet of paper over the tray for
everyone, lifts it with the print on it, and hangs it in the gallery.

**The gallery** is a wall of prints on clips, each slightly askew. A print's
page shows it large with its recipe written out ("212 drops of ink, 14 of them
inside others … 11 passes of the comb") and a button to watch it being made, which runs the
recipe through the same worker at a steady pace.

## Running it

You need Rust (stable, with the `wasm32-unknown-unknown` target) and Node
20.19+.

```console
make setup         # build the tray to WebAssembly, npm install
make dev           # server on :8740, workshop on :5174 (Vite proxies /api and /ws)
make test          # Rust tests, clippy, vue-tsc
make determinism   # native and WebAssembly replay a print bit for bit
make build         # the web app in web/dist, the server in release
make serve         # one process: the server hosting the built workshop on :8740
```

Or by hand:

```console
cargo run --release -p ebru-server
cd web && npm install && npm run wasm && npm run dev
```

Configuration is by environment variable:

| | |
| --- | --- |
| `EBRU_ADDR` | where to listen, default `127.0.0.1:8740` |
| `EBRU_DATA` | prints, recipes and the gallery database, default `data` |
| `EBRU_WEB` | a directory of built files to host, for single-process deployment |
| `EBRU_IDLE` | seconds of quiet before the tray marbles by itself, or `off`; default 20 |

The server tests start the whole server on a free port with its own data
directory, and drive it with real WebSocket clients: two people working, a
refusal, a sheet laid through to its PNG in the gallery.

## Layout

```
crates/
  marbling/        the tray: drops, strokes, combs, swirls, snapshots
    tests/         area, exactness, determinism against a recorded print
  wasm/            the tray for the browser, as bare exports
  server/
    src/
      workshop.rs  the thread that owns the tray, rate limits, the bot
      patterns.rs  tally, traditional names, compositions
      print.rs     rendering onto paper
      store.rs     SQLite and the print files
      ws.rs        connections, cursors, presence
      http.rs      the API and static hosting
    tests/         the whole server over WebSockets; pigments match the web app
web/
  scripts/         build the WebAssembly; the determinism check
  src/
    lib/
      tray.worker.ts  the tray in a worker, frames out
      tray.ts         the shared tray: socket, resync, presence
      draw.ts         outlines onto a canvas
    components/
      TrayCanvas.vue  the bath, the tools, the overlay
      Rail.vue        tools, pigment pots, sizes, the sheet
    views/
      TrayView.vue    the workshop
      PrintsView.vue  the gallery wall
      PrintView.vue   one print, its recipe, its replay
```
