# Build stage
FROM rust:1.95-slim-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
RUN useradd -m -s /bin/bash oxivend
COPY --from=builder /app/target/release/oxivend /usr/local/bin/
USER oxivend
EXPOSE 3000
CMD ["oxivend"]
