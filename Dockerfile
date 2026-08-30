# Build stage
FROM rust:1.89-bookworm AS builder

WORKDIR /usr/src/ruststack
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build --release --bin ruststack

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/ruststack/target/release/ruststack /usr/local/bin/ruststack

ENV PORT=4566
ENV HOST=0.0.0.0
ENV SERVICES=s3,sqs

EXPOSE 4566

ENTRYPOINT ["ruststack"]
