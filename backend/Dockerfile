# syntax=docker/dockerfile:1.4

FROM rust:1.84-bookworm AS chef
RUN rustup default nightly-2026-01-28 && cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Cache mounts persist on the builder's filesystem across builds,
# surviving Docker layer cache invalidation
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release && \
    cp target/release/yomitaisen /tmp/yomitaisen

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /tmp/yomitaisen ./yomitaisen
COPY --from=builder /app/migrations ./migrations

ENV PORT=3000
ENV DATABASE_URL=sqlite:/app/data/data.db?mode=rwc
ENV RUST_LOG=yomitaisen=info

EXPOSE 3000
CMD ["./yomitaisen"]
