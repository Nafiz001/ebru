# syntax=docker/dockerfile:1
#
# The whole workshop in one image: the Rust server, hosting the Vue app,
# which runs the same marbling crate compiled to WebAssembly.
#
#   docker build -t ebru .
#   docker run -p 8740:8740 -v ebru-data:/data ebru

# --- Rust: the server, and the tray for the browser -------------------------
FROM rust:1-slim-trixie AS rust
WORKDIR /src
RUN rustup target add wasm32-unknown-unknown
COPY Cargo.toml Cargo.lock ./
COPY crates crates
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p ebru-wasm --target wasm32-unknown-unknown \
 && cargo build --release --locked -p ebru-server \
 && mkdir -p /out \
 && cp target/wasm32-unknown-unknown/release/ebru_wasm.wasm /out/ebru.wasm \
 && cp target/release/ebru /out/ebru

# --- The Vue app, with the tray copied in where Vite serves it ---------------
FROM node:22-slim AS web
WORKDIR /web
COPY web/package.json web/package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY web .
COPY --from=rust /out/ebru.wasm public/ebru.wasm
RUN npm run build

# --- Runtime ------------------------------------------------------------------
FROM debian:trixie-slim
RUN useradd --system --uid 10001 --home-dir /data ebru \
 && mkdir /data && chown ebru /data
COPY --from=rust /out/ebru /usr/local/bin/ebru
COPY --from=web /web/dist /srv/web
ENV EBRU_ADDR=0.0.0.0:8740 \
    EBRU_DATA=/data \
    EBRU_WEB=/srv/web
USER ebru
VOLUME /data
EXPOSE 8740
CMD ["ebru"]
