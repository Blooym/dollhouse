# ----------
#   SETUP
# ----------
FROM alpine:latest AS setup
RUN adduser -S -s /bin/false -D dollhouse
RUN mkdir /dir

# -----------
#    BUILD
# -----------
FROM rust:1-alpine AS build
WORKDIR /build
RUN apk add --no-cache --update build-base

# Pre-cache dependencies
COPY ["Cargo.toml", "Cargo.lock", "./"]
RUN mkdir src \
    && echo "// Placeholder" > src/lib.rs \
    && cargo build --release \
    && rm src/lib.rs

# Build
COPY src ./src
RUN cargo build --release

# -----------
#   RUNTIME
# -----------
FROM scratch
WORKDIR /opt

COPY --from=build /build/target/release/dollhouse /usr/bin/dollhouse

# Setup deployment image.
COPY --from=setup /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=setup /etc/passwd /etc/passwd
COPY --from=setup /bin/false /bin/false
USER dollhouse
COPY --from=setup --chown=dollhouse /dir /srv/dollhouse

# Set configuration defaults for container builds.
ENV DOLLHOUSE_ADDRESS=0.0.0.0:8731
ENV DOLLHOUSE_PUBLIC_URL=http://0.0.0.0:8731
ENV DOLLHOUSE_STORAGE_PROVIDER=fs:///srv/dollhouse
ENV RUST_LOG=info
EXPOSE 8731

ENTRYPOINT ["/usr/bin/dollhouse"]