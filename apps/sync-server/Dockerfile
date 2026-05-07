# Build: docker build -f apps/sync-server/Dockerfile -t slop-sync-server .
# Run:   docker run -p 7878:7878 -v $PWD/data:/data slop-sync-server
FROM rust:1.82-slim AS builder
WORKDIR /work
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev cmake clang && \
    rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY apps/ apps/
COPY packages/ packages/
RUN cargo build --release --bin slop-sync-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /work/target/release/slop-sync-server /usr/local/bin/slop-sync-server
EXPOSE 7878
VOLUME ["/data"]
ENV RUST_LOG=info
ENTRYPOINT ["slop-sync-server", "--bind", "0.0.0.0:7878", "--data-dir", "/data"]
