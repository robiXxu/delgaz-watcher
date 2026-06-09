FROM rust:1-bookworm AS builder

WORKDIR /app

# Cache dependencies first
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/delgaz-watcher /app/delgaz-watcher

CMD ["/app/delgaz-watcher"]
