# Build stage
FROM rust:latest AS builder

WORKDIR .
COPY . .

RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /target/release/iota /usr/local/bin/

EXPOSE 1984

CMD ["iota"]