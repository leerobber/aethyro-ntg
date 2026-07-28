# Aethyro NTG kernel — multi-stage container image.
#
# Stage 1: build the Rust kernel (release, stripped).
# Stage 2: minimal Debian runtime — no build tools, no Rust toolchain.
#
# Binaries included:
#   kernel_host      — NanoKeymaster routing agent
#   vitascale_demo   — VITASCALE L0 demo (development/smoke)
#
# Environment variables:
#   KEYMASTER_BACKEND_URL — external routing backend (optional)
#   RUST_LOG              — tracing level (default: info)

FROM rust:1.79-bookworm AS builder

WORKDIR /build

# Cache dependency compilation layer.
COPY kernel/Cargo.toml kernel/Cargo.lock ./kernel/
RUN mkdir -p kernel/src && echo "fn main(){}" > kernel/src/main.rs \
 && cd kernel && cargo build --release 2>/dev/null || true \
 && rm -f kernel/src/main.rs

# Build full sources.
COPY kernel/src ./kernel/src
RUN cd kernel && cargo build --release --bins \
 && strip target/release/kernel_host target/release/vitascale_demo 2>/dev/null || true

# ---------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

RUN apt-get update -qq \
 && apt-get install -y --no-install-recommends ca-certificates libssl3 \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /build/kernel/target/release/kernel_host    ./kernel_host
COPY --from=builder /build/kernel/target/release/vitascale_demo ./vitascale_demo

ENV RUST_LOG=info
ENV KEYMASTER_BACKEND_URL=""

EXPOSE 8080

CMD ["./kernel_host"]
