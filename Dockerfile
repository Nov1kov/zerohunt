# Stage 1: Builder
FROM rust:1.81.0 as builder

WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock ./

RUN mkdir src
RUN echo "fn main() {}" > src/main.rs

RUN cargo build --release
RUN rm -f target/release/deps/app* && rm -r src

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