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
COPY crates/executor/Cargo.toml crates/executor/Cargo.toml

# [============= Copy crate and local deps configs =============]
RUN mkdir -p crates/executor/src crates/shared_types/src crates/utils/src && \
    echo 'fn main() {}' > crates/executor/src/main.rs && \
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
COPY crates/executor ./crates/executor

RUN cargo build --release --bin executor

FROM alpine

COPY --from=builder /app/target/release/executor /usr/local/bin/executor

ENV RUST_LOG=DEBUG

CMD ["executor"]