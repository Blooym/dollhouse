# ----------
#    USER
# ----------
FROM alpine:latest AS user
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
FROM scratch AS runtime
WORKDIR /opt

COPY --from=build /build/target/release/dollhouse /usr/bin/dollhouse

# Import and switch to non-root user.
COPY --from=user /etc/passwd /etc/passwd
COPY --from=user /bin/false /bin/false
USER dollhouse
COPY --from=user --chown=dollhouse /dir /srv/dollhouse

ENV DOLLHOUSE_ADDRESS=0.0.0.0:8731
ENV DOLLHOUSE_PUBLIC_URL=http://0.0.0.0:8731
ENV DOLLHOUSE_STORAGE_PROVIDER=fs:///srv/dollhouse
ENV RUST_LOG=info
EXPOSE 8731

ENTRYPOINT ["/usr/bin/dollhouse"]