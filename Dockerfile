FROM alpine:3.21

ARG TARGETARCH

RUN apk add --no-cache ca-certificates curl

WORKDIR /app
COPY target/bin-${TARGETARCH}/ruststack /usr/local/bin/ruststack
RUN chmod +x /usr/local/bin/ruststack

EXPOSE 4566

ENV HOST=0.0.0.0
ENV PORT=4566

HEALTHCHECK --interval=5s --timeout=3s --start-period=2s --retries=3 \
  CMD curl -f http://localhost:4566/_ruststack/health || exit 1

ENTRYPOINT ["/usr/local/bin/ruststack"]
CMD []
