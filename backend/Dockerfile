# Build Stage
FROM rust:1.81-slim-bookworm as builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev
COPY . .
RUN cargo build --release

# Runtime Stage
FROM debian:bookworm-slim
WORKDIR /app
# Install FFmpeg and OpenSSL (Required for Sonica)
RUN apt-get update && apt-get install -y ffmpeg openssl ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/sonica-backend .
COPY --from=builder /app/songs.db .

# Expose the port (Hugging Face uses 7860 by default)
ENV PORT=7860
EXPOSE 7860

CMD ["./sonica-backend"]
