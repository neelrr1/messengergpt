FROM rust:latest as builder
WORKDIR /usr/src/messengergpt
COPY . .
RUN cargo install --path .

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/messengergpt /usr/local/bin/messengergpt

# Changed to bind mount
# COPY --from=builder /usr/src/messengergpt/.env /usr/local/.env

WORKDIR /usr/local
CMD ["messengergpt"]