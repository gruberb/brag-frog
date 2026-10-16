FROM rust:1.93-slim AS builder
WORKDIR /app

# 1) Copy only manifests and create a dummy main/lib to compile dependencies
COPY Cargo.toml Cargo.lock ./
COPY apps/api/Cargo.toml apps/api/Cargo.toml
RUN mkdir -p apps/api/src && echo "fn main() {}" > apps/api/src/main.rs && touch apps/api/src/lib.rs
RUN cargo build --release -p brag-frog && rm -rf apps/api/src

# 2) Copy actual source and migrations, then build (only app code recompiles)
COPY apps/api/migrations/ apps/api/migrations/
COPY apps/api/src/ apps/api/src/
RUN touch apps/api/src/main.rs apps/api/src/lib.rs && cargo build --release -p brag-frog

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/brag-frog /usr/local/bin/
COPY apps/api/templates/ /app/templates/
COPY apps/api/static/ /app/static/
COPY apps/api/migrations/ /app/migrations/
COPY apps/api/config/ /app/config/
WORKDIR /app
ENV PORT=8080
ENV BRAGFROG_DATABASE_PATH=/data/bragfrog.db
EXPOSE 8080
CMD ["brag-frog"]
