FROM rust:1.88-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/flowfull-rust-starter /usr/local/bin/flowfull-rust-starter

ENV HOST=0.0.0.0
ENV PORT=3001
EXPOSE 3001

CMD ["flowfull-rust-starter"]
