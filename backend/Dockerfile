# syntax=docker/dockerfile:1.4

# Build stage
FROM rust:1.84-bookworm AS builder
RUN rustup default nightly-2026-01-28
WORKDIR /app

# 1. Copy manifests only (cached unless deps change)
COPY Cargo.toml Cargo.lock ./

# 2. Create dummy lib.rs + main.rs and build dependencies
RUN mkdir src && echo '' > src/lib.rs && echo 'fn main() {}' > src/main.rs
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release

# 3. Remove dummy artifacts that would conflict, copy real source
RUN rm -rf src target/release/.fingerprint target/release/deps/yomitaisen* target/release/yomitaisen*
COPY src ./src

# 4. Build real binary (deps already compiled)
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release

# Runtime stage
FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/yomitaisen ./yomitaisen
COPY migrations ./migrations

ENV PORT=3000
ENV DATABASE_URL=sqlite:/app/data/data.db?mode=rwc
ENV RUST_LOG=yomitaisen=info

EXPOSE 3000
CMD ["./yomitaisen"]
