# Image for GoReleaser builds.

FROM gcr.io/distroless/cc-debian12:latest

ARG TARGETPLATFORM
COPY ${TARGETPLATFORM}/yokoku /app/yokoku
COPY target/web/public /app/public
COPY config/docker.toml /config/app.toml

EXPOSE 8080
VOLUME ["/data"]
ENTRYPOINT ["/app/yokoku", "--config", "/config/app.toml"]
