FROM rust:1.87-alpine AS base

ARG BIN_NAME

RUN rustup target add x86_64-unknown-linux-musl

RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static

FROM base AS deps

WORKDIR /app

# [============= Copy workspace config =============]
COPY Cargo.toml Cargo.lock ./

# [============= Copy crate and local deps configs =============]
COPY crates/utils/Cargo.toml crates/utils/Cargo.toml
COPY crates/shared_types/Cargo.toml crates/shared_types/Cargo.toml
COPY crates/binance_connector/Cargo.toml crates/binance_connector/Cargo.toml
COPY crates/${BIN_NAME}/Cargo.toml crates/${BIN_NAME}/Cargo.toml

# [============= Copy crate and local deps configs =============]
RUN mkdir -p crates/${BIN_NAME}/src crates/shared_types/src crates/utils/src crates/binance_connector/src && \
    echo 'fn main() {}' > crates/${BIN_NAME}/src/main.rs && \
    echo '' > crates/binance_connector/src/lib.rs && \
    echo '' > crates/shared_types/src/lib.rs && \
    echo '' > crates/utils/src/lib.rs

# [============= Cache workspace dependencies =============]
RUN cargo build --workspace --release

FROM base AS builder

WORKDIR /app

COPY --from=deps /app/target target
COPY --from=deps /usr/local/cargo /usr/local/cargo

COPY Cargo.toml Cargo.lock ./

COPY crates/utils ./crates/utils
COPY crates/shared_types ./crates/shared_types
COPY crates/binance_connector ./crates/binance_connector
COPY crates/${BIN_NAME} ./crates/${BIN_NAME}

RUN cargo build --release --bin ${BIN_NAME}

FROM alpine

ARG BIN_NAME

COPY --from=builder /app/target/release/${BIN_NAME} /usr/local/bin/app

ENV RUST_LOG=DEBUG

CMD ["/usr/local/bin/app"]