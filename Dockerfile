# Stage 1: Builder
FROM rust:1.82.0 as builder

WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock ./

RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src && rm -rf target/release/build

COPY . .

RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/zerohunt /usr/local/bin/zerohunt

WORKDIR /app

ENTRYPOINT ["zerohunt"]
