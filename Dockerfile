# 1) Web app
FROM node:24-alpine AS web
WORKDIR /repo
COPY package.json package-lock.json ./
COPY apps/web/package.json apps/web/package.json
RUN npm ci
COPY apps/web apps/web
RUN npm run build -w apps/web

# 2) API. Dependencies compile against dummy sources first so code changes reuse the cache.
FROM rust:1.93-slim AS api
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY apps/api/Cargo.toml apps/api/Cargo.toml
RUN mkdir -p apps/api/src && echo "fn main() {}" > apps/api/src/main.rs && touch apps/api/src/lib.rs
RUN cargo build --release -p brag-frog && rm -rf apps/api/src
COPY apps/api/migrations/ apps/api/migrations/
COPY apps/api/src/ apps/api/src/
RUN touch apps/api/src/main.rs apps/api/src/lib.rs && cargo build --release -p brag-frog

# 3) Runtime
FROM debian:trixie-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=api /app/target/release/brag-frog /usr/local/bin/
COPY --from=web /repo/apps/web/dist /app/web
COPY apps/api/config/ /app/config/
WORKDIR /app
ENV PORT=8080 \
    BRAGFROG_DATABASE_PATH=/data/bragfrog.db \
    BRAGFROG_WEB_DIST=/app/web \
    BRAGFROG_CONFIG_DIR=/app/config \
    BRAGFROG_CUSTOM_DIR=/app/custom
EXPOSE 8080
CMD ["brag-frog"]
