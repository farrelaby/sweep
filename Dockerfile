# Using latest stable (edition 2024 requires Rust ≥1.85)
FROM rust:alpine AS build
RUN apk add --no-cache musl-dev
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked

FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=build /build/target/release/dirsweep /usr/local/bin/dirsweep
ENTRYPOINT ["dirsweep"]
