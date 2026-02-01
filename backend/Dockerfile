FROM rustlang/rust:nightly AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy manifests first for dependency caching
COPY Cargo.toml Cargo.lock ./

# Create dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy actual source code
COPY src ./src
COPY migrations ./migrations

# Touch main.rs to invalidate the dummy build
RUN touch src/main.rs

# Build the actual application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy the binary
COPY --from=builder /app/target/release/yomitaisen .

# Copy migrations (needed at runtime for sqlx::migrate!)
COPY --from=builder /app/migrations ./migrations

ENV PORT=3000
ENV DATABASE_URL=sqlite:/app/data/data.db?mode=rwc
ENV RUST_LOG=yomitaisen=info

EXPOSE 3000

CMD ["./yomitaisen"]
