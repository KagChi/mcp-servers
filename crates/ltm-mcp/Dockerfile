# Build stage
FROM rust:1.95-bookworm as builder

# Verify Rust version
RUN rustc --version

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy workspace files
COPY Cargo.toml ./
COPY crates/mcp-common crates/mcp-common
COPY crates/ltm-mcp crates/ltm-mcp

# Build the application
RUN cargo build --release --package ltm-mcp

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -u 1000 -m ltm

# Create app directory
WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/ltm-mcp /usr/local/bin/ltm-mcp

# Copy migrations
COPY --from=builder /app/crates/ltm-mcp/migrations /app/migrations

# Set ownership
RUN chown -R ltm:ltm /app

# Switch to non-root user
USER ltm

# Health check - verify HTTP server is responding
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/mcp || exit 1

# Expose port
EXPOSE 3000

# Run the application
CMD ["/usr/local/bin/ltm-mcp"]
