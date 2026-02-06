# --- Build stage ---
FROM rust:1-alpine AS chef
RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static curl
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release

# --- Server target ---
FROM alpine:3.21 AS server
RUN apk add --no-cache ca-certificates curl
COPY --from=builder /app/target/release/meddler-server /usr/local/bin/
EXPOSE 3000
CMD ["meddler-server"]

# --- Agent target ---
FROM alpine:3.21 AS agent
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/target/release/meddler /usr/local/bin/
CMD ["meddler", "agent"]
