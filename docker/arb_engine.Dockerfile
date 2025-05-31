FROM rust:1.87-alpine AS base

RUN rustup target add x86_64-unknown-linux-musl

RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static

FROM base as deps

WORKDIR /app

# [============= Copy workspace config =============]
COPY Cargo.toml Cargo.lock ./

# [============= Copy crate and local deps configs =============]
COPY crates/utils/Cargo.toml crates/utils/Cargo.toml
COPY crates/shared_types/Cargo.toml crates/shared_types/Cargo.toml
COPY crates/binance_connector/Cargo.toml crates/binance_connector/Cargo.toml
COPY crates/arb_engine/Cargo.toml crates/arb_engine/Cargo.toml

# [============= Copy crate and local deps configs =============]
RUN mkdir -p crates/arb_engine/src crates/utils/src crates/shared_types/src crates/binance_connector/src && \
    echo 'fn main() {}' > crates/arb_engine/src/main.rs && \
    echo '' > crates/binance_connector/src/lib.rs && \
    echo '' > crates/shared_types/src/lib.rs && \
    echo '' > crates/utils/src/lib.rs

# [============= Cache workspace dependencies =============]
RUN cargo build --workspace --release

FROM base as builder

WORKDIR /app

COPY --from=deps /app/target target
COPY --from=deps /usr/local/cargo /usr/local/cargo

COPY Cargo.toml Cargo.lock ./

COPY crates/utils ./crates/utils
COPY crates/shared_types ./crates/shared_types
COPY crates/binance_connector ./crates/binance_connector
COPY crates/arb_engine ./crates/arb_engine

RUN cargo build --release --bin arb_engine

FROM alpine

COPY --from=builder /app/target/release/arb_engine /usr/local/bin/arb_engine

ENV RUST_LOG=DEBUG

CMD ["arb_engine"]