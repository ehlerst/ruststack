# Multi-stage Docker build for RustStack
FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev pkgconfig

WORKDIR /app
COPY . .

RUN cargo build --release --bin ruststack -p ruststack-server

FROM alpine:3.21

RUN apk add --no-cache ca-certificates curl

WORKDIR /app
COPY --from=builder /app/target/release/ruststack /usr/local/bin/ruststack

EXPOSE 4566

ENV HOST=0.0.0.0
ENV PORT=4566

HEALTHCHECK --interval=5s --timeout=3s --start-period=2s --retries=3 \
  CMD curl -f http://localhost:4566/_ruststack/health || exit 1

ENTRYPOINT ["/usr/local/bin/ruststack"]
CMD []
