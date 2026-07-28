# Multi-stage build for optimal image size
FROM rust:1.97 as builder

WORKDIR /workspace

# Copy source
COPY . .

# Build release binary
RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /workspace/target/release/hostframe-backend /app/hostframe-backend

# Cloud Run configuration
ENV PORT=8080

EXPOSE 8080

ENTRYPOINT ["/app/hostframe-backend"]
