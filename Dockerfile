# syntax=docker/dockerfile:1

FROM --platform=$BUILDPLATFORM amd64/rust:slim-bookworm AS builder

WORKDIR /src
COPY src ./src
COPY config ./config
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release
RUN ls target/release

FROM scratch AS exporter
WORKDIR /app
COPY --from=builder /src/target/release/jarvis-agent ./
