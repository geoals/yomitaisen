# syntax=docker/dockerfile:1.4

# Build stage
FROM rust:1.84-bookworm AS builder
RUN rustup default nightly-2026-01-28
WORKDIR /app

# 1. Copy manifests only (cached unless deps change)
COPY Cargo.toml Cargo.lock ./

# 2. Create dummy main.rs and build dependencies
RUN mkdir src && echo 'fn main() {}' > src/main.rs
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release

# 3. Remove dummy, copy real source
RUN rm -rf src
COPY src ./src

# 4. Touch main.rs to invalidate cache, build real binary
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    touch src/main.rs && \
    cargo build --release && \
    cp target/release/yomitaisen /app/yomitaisen-bin

# Runtime stage
FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/yomitaisen-bin ./yomitaisen
COPY migrations ./migrations

ENV PORT=3000
ENV DATABASE_URL=sqlite:/app/data/data.db?mode=rwc
ENV RUST_LOG=yomitaisen=info

EXPOSE 3000
CMD ["./yomitaisen"]
