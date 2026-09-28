# Image for GoReleaser builds.

FROM gcr.io/distroless/cc-debian12:latest

ARG TARGETPLATFORM
COPY ${TARGETPLATFORM}/yokoku /app/yokoku

ENV APP__DATABASE__PATH=/data/yokoku.db

VOLUME ["/data"]
ENTRYPOINT ["/app/yokoku"]
