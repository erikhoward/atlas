# Atlas - OpenEHR to Azure Cosmos DB ETL Tool
# Multi-stage Docker build for optimized image size

# ============================================================================
# Stage 1: Builder - Compile the Rust application
# ============================================================================
FROM rust:latest AS builder

# Set working directory
WORKDIR /usr/src/atlas

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code and migrations (migrations are embedded at compile time via include_str!)
COPY src ./src
COPY migrations ./migrations

# Build the application in release mode
# This produces an optimized binary with LTO and other optimizations
# Note: We only build the main binary, not tests or examples
RUN cargo build --release --bin atlas

# ============================================================================
# Stage 2: Runtime - Minimal image with only the binary
# ============================================================================
FROM debian:bookworm-slim

# Install runtime dependencies
# - ca-certificates: Required for HTTPS connections to Azure and OpenEHR servers
# - libssl3: Required for TLS/SSL support
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 && \
    rm -rf /var/lib/apt/lists/*

# Create a non-root user for running the application
RUN useradd -m -u 1000 atlas

# Set working directory
WORKDIR /app

# Copy the compiled binary from builder stage
COPY --from=builder /usr/src/atlas/target/release/atlas /usr/local/bin/atlas

# Create directories for configuration and logs
RUN mkdir -p /app/config /app/logs && \
    chown -R atlas:atlas /app

# Switch to non-root user
USER atlas

# Set environment variables
ENV RUST_LOG=info

# The application expects a configuration file
# Users should mount their config file to /app/config/atlas.toml
# Example: docker run -v $(pwd)/atlas.toml:/app/config/atlas.toml atlas

# Expose no ports (this is a CLI tool, not a server)

# Set the entrypoint to the atlas binary
ENTRYPOINT ["/usr/local/bin/atlas"]

# Default command shows help
CMD ["--help"]

