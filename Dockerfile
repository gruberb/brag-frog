FROM rust:1.93-slim AS builder
WORKDIR /app

# 1) Copy only manifests and create a dummy main/lib to compile dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && touch src/lib.rs
RUN cargo build --release && rm -rf src

# 2) Copy actual source and migrations, then build (only app code recompiles)
COPY migrations/ migrations/
COPY src/ src/
RUN touch src/main.rs src/lib.rs && cargo build --release

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/brag-frog /usr/local/bin/
COPY templates/ /app/templates/
COPY static/ /app/static/
COPY migrations/ /app/migrations/
COPY config/ /app/config/
WORKDIR /app
ENV PORT=8080
ENV BRAGFROG_DATABASE_PATH=/data/bragfrog.db
EXPOSE 8080
CMD ["brag-frog"]
