# HouseCheck API — multi-stage build. The serving DB is a read-only artifact baked into
# the image, so the running container needs NO secrets (ingest is done ahead of time).
# Build the DB first:  cargo run -p ingest -- --real --cd 303 --limit 250 --out data/housecheck.db
FROM rust:1-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
# rusqlite `bundled` compiles SQLite from C source — the full rust image ships gcc.
RUN cargo build --release -p api --locked

FROM debian:bookworm-slim
# ca-certificates for outbound HTTPS (the /search endpoint calls NYC GeoSearch).
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /app/target/release/housecheck-api /usr/local/bin/housecheck-api
# Bake the prebuilt read-only Building-Health DB into the image.
COPY data/housecheck.db /app/data/housecheck.db
ENV HOUSECHECK_DB=/app/data/housecheck.db \
    HOST=0.0.0.0 \
    PORT=8080
EXPOSE 8080
CMD ["housecheck-api"]
