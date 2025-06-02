FROM rust:1.87.0 AS base

ARG BIN_NAME

RUN cargo install cargo-chef

FROM base AS planner

WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json


FROM base AS builder

WORKDIR /app

COPY --from=planner /app/recipe.json recipe.json

RUN cargo chef cook --release --recipe-path recipe.json

#COPY --from=deps /app/target target
#COPY --from=deps /usr/local/cargo /usr/local/cargo

COPY . .

#COPY crates/utils ./crates/utils
#COPY crates/shared_types ./crates/shared_types
#COPY crates/binance_connector ./crates/binance_connector
#COPY crates/${BIN_NAME} ./crates/${BIN_NAME}

RUN cargo build --release --bin ${BIN_NAME}

FROM gcr.io/distroless/cc-debian12

ARG BIN_NAME

COPY --from=builder /app/target/release/${BIN_NAME} /app

ENV RUST_LOG=INFO

CMD ["/app"]