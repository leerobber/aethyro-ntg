# Multi-stage build for Aethyro NTG

# Stage 1: Builder
FROM rust:1.75-slim-bookworm as builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy Cargo manifests
COPY kernel/Cargo.toml kernel/Cargo.lock ./

# Copy source code
COPY kernel/src ./src

# Build release binary
RUN cargo build --release && \
    mv target/release/kernel_host /usr/local/bin/

# Stage 2: Runtime
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    tini \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 aethyro

# Copy binary from builder
COPY --from=builder /usr/local/bin/kernel_host /usr/local/bin/

# Create directories
RUN mkdir -p /etc/aethyro /var/lib/aethyro/models /var/lib/aethyro/ledger /var/lib/aethyro/cache && \
    chown -R aethyro:aethyro /var/lib/aethyro /etc/aethyro

# Copy configuration template
COPY docs/aethyro.yaml /etc/aethyro/aethyro.yaml.template

# Set working directory
WORKDIR /var/lib/aethyro

# Switch to non-root user
USER aethyro

# Expose ports
EXPOSE 8080 9090

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Use tini as init process
ENTRYPOINT ["/usr/bin/tini", "--"]

# Default command
CMD ["kernel_host", "--config", "/etc/aethyro/aethyro.yaml"]
