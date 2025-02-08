# ==========================
# 🛠 1️⃣ Build Stage (Compile Rust)
# ==========================
FROM rust:alpine AS builder

# Install necessary build dependencies for OpenSSL and Rust
RUN apk add --no-cache musl-dev openssl-dev pkgconfig

# Set work directory inside the container
WORKDIR /app

# Cache dependencies to optimize builds
COPY Cargo.toml Cargo.lock ./
RUN cargo fetch

# Copy application source code
COPY src ./src

# Build release binary
RUN cargo build --all --release

# ==========================
# 🚀 2️⃣ Runtime Stage (Minimal Alpine with SSL Support)
# ==========================
FROM alpine:latest

# Install only necessary runtime dependencies
RUN apk add --no-cache libgcc openssl ca-certificates

# Set working directory
WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/server /app/server



# Expose HTTPS port (Change if needed)
EXPOSE 8888

ENV RUST_LOG=debug
# Run the Rust API Server with SSL
CMD ["/app/server"]