FROM gcr.io/distroless/cc-debian12:latest

ARG TARGETARCH

WORKDIR /app
COPY docker-bin/${TARGETARCH}/ruststack /usr/local/bin/ruststack

EXPOSE 4566

ENV HOST=0.0.0.0
ENV PORT=4566

ENTRYPOINT ["/usr/local/bin/ruststack"]
CMD []
