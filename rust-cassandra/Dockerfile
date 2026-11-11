# syntax=docker/dockerfile:1
# One image with both binaries. Compose picks which one to run via `command`.

FROM rust:1.92-bookworm AS build
WORKDIR /src
COPY . .
# Cache mounts keep the crate registry and target dir between builds,
# so changing one line of Rust doesn't recompile every dependency.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked \
    && cp target/release/parkd target/release/sim /usr/local/bin/

FROM debian:bookworm-slim
# curl is only here for the parkd healthcheck.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /usr/local/bin/parkd /usr/local/bin/sim /usr/local/bin/
