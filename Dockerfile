FROM rust:latest as builder
WORKDIR /usr/src/messengergpt
COPY . .
RUN cargo install --path .

FROM debian:bullseye-slim
COPY --from=builder /usr/local/cargo/bin/messengergpt /usr/local/bin/messengergpt

# Changed to bind mount
# COPY --from=builder /usr/src/messengergpt/.env /usr/local/.env

WORKDIR /usr/local
CMD ["messengergpt"]