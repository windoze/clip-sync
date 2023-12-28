FROM messense/rust-musl-cross:x86_64-musl AS builder
WORKDIR /usr/src/
COPY . ./
RUN cargo build --release --target=x86_64-unknown-linux-musl

FROM alpine
RUN apk add --update openssl bash
COPY --from=builder /usr/src/target/x86_64-unknown-linux-musl/release/clip-sync /app/clip-sync
COPY --from=builder /usr/src/config.toml /config/config.toml
EXPOSE 3000
WORKDIR /app
CMD ["/app/clip-sync", "--config", "/config/config.toml"]
