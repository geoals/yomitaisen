# syntax=docker/dockerfile:1.4

# Stage 1: Install cargo-chef with pinned nightly
FROM rust:1.84-bookworm AS chef
RUN rustup default nightly-2026-01-28
RUN cargo install cargo-chef
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app

# Stage 2: Generate dependency recipe
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Build dependencies (cached) then application
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release && \
    cp target/release/yomitaisen /app/yomitaisen-bin

# Stage 4: Runtime
FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/yomitaisen-bin ./yomitaisen
COPY --from=builder /app/migrations ./migrations

ENV PORT=3000
ENV DATABASE_URL=sqlite:/app/data/data.db?mode=rwc
ENV RUST_LOG=yomitaisen=info

EXPOSE 3000

CMD ["./yomitaisen"]
