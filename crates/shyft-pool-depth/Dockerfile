FROM rust:1.75-slim as builder

WORKDIR /app
COPY . .

# Build the shyft-pool-depth binary
RUN cd crates/shyft-pool-depth && cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from the builder stage
COPY --from=builder /app/crates/shyft-pool-depth/target/release/shyft-pool-depth /usr/local/bin/

# Create a non-root user
RUN useradd -m -u 1000 app && chown -R app:app /app
USER app

EXPOSE 8080

CMD ["/usr/local/bin/shyft-pool-depth"]
