# Build Stage
FROM rust:1.92-slim-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release --locked

# Runtime Stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ffmpeg && rm -rf /var/lib/apt/lists/*

# Hugging Face runs the container as uid 1000. The server writes songs/, temp/
# and SQLite's -wal/-shm files next to itself, so that user must own the folder.
RUN useradd -m -u 1000 user
USER user
WORKDIR /home/user/app
COPY --from=builder --chown=user /app/target/release/sonica-backend ./
COPY --chown=user songs.db ./

# Expose the port (Hugging Face uses 7860 by default)
ENV PORT=7860
EXPOSE 7860

CMD ["./sonica-backend"]
