FROM rust:1.86-slim-bookworm AS builder
WORKDIR /app
COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release --locked 2>/dev/null || cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/qi-cloud /usr/local/bin/qi-cloud
ENV QI_CLOUD_BIND=0.0.0.0:8788
ENV QI_CLOUD_DB=/var/lib/qi-cloud/qi_control_plane.sqlite3
VOLUME ["/var/lib/qi-cloud"]
EXPOSE 8788
CMD ["qi-cloud"]
