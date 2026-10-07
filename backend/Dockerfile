# Build stage for Rust
FROM rust:1.97.1-slim-bookworm AS rust-builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY vendor ./vendor

# Copy source code
COPY src ./src
COPY migrations ./migrations
COPY locales ./locales

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies. perl-base is pinned rather than upgraded with a
# bare `apt-get upgrade` so the image stays a function of this tree instead of
# mirror state: the s9pk is signed over its hash, so two builds of one commit
# must not differ. The base image ships 5.36.0-7+deb12u3, which carries 7 CVEs
# (3 critical); the fix lives only in bookworm-security, not in any base tag.
# When Debian publishes a later revision the mirror drops this one and the build
# fails loudly, which is the point: bump the pin and re-run the container scan.
RUN apt-get update && apt-get install -y --only-upgrade \
    perl-base=5.36.0-7+deb12u4 \
    && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpcre2-8-0 \
    && rm -rf /var/lib/apt/lists/*

# Copy the Rust binary from builder stage
COPY --from=rust-builder /app/target/release/canary /app/canary

# Migrations and application artifacts remain read-only to the runtime user.
# Force world-readable copies so a 0600 build-context file cannot skip startup.
COPY --from=rust-builder /app/migrations /app/migrations
RUN chmod -R a+rX /app/migrations

# Create a dedicated runtime user with owner-only application data.
RUN groupadd --gid 1000 canary \
    && useradd --uid 1000 --gid canary --create-home --shell /usr/sbin/nologin canary \
    && mkdir -p /app/data /app/logs \
    && chown canary:canary /app/data /app/logs \
    && chmod 700 /app/data /app/logs

USER canary

# Expose port
EXPOSE 3000

# Run the application
CMD ["./canary"]
