FROM rust:buster AS builder
WORKDIR /usr/src/clip-sync
COPY . .
RUN apt-get update && apt-get install -y musl-tools musl-dev \
    && rustup target add x86_64-unknown-linux-musl \
    && rustup target add wasm32-unknown-unknown \
    && cargo install dioxus-cli
RUN cargo build --release  --no-default-features --features server-only --target x86_64-unknown-linux-musl \
    && cd clip-sync-ui && dx build --release

FROM alpine
RUN apk add --update openssl bash
RUN mkdir /index
COPY --from=builder /usr/src/clip-sync/target/x86_64-unknown-linux-musl/release/clip-sync /app/clip-sync
COPY --from=builder /usr/src/clip-sync/clip-sync-ui/dist /static-files
COPY --from=builder /usr/src/clip-sync/config.toml /config/config.toml
EXPOSE 3000
WORKDIR /
CMD ["/app/clip-sync", "--config", "/config/config.toml"]
