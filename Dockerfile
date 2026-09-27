# Image for GoReleaser builds: the binary with its web asset bundle beside it.

FROM gcr.io/distroless/cc-debian12:latest

ARG TARGETPLATFORM
COPY ${TARGETPLATFORM}/yokoku /app/yokoku
COPY target/yokoku-assets /app/assets

ENV HOST=0.0.0.0 \
    PORT=3000 \
    APP__DATABASE__PATH=/data/yokoku.db

VOLUME ["/data"]
EXPOSE 3000
ENTRYPOINT ["/app/yokoku"]
