FROM --platform=$BUILDPLATFORM tonistiigi/xx AS xx
ARG TARGETARCH

FROM messense/rust-musl-cross:x86_64-musl-amd64 AS builder-amd64
FROM messense/rust-musl-cross:aarch64-musl-amd64 AS builder-arm64

FROM builder-${TARGETARCH} as builder
ARG TARGETARCH
ARG TARGETPLATFORM
COPY --from=xx / /
WORKDIR /usr/src/
COPY . ./
RUN cargo build --release --no-default-features --features server-only --target=$(xx-info march)-unknown-linux-musl && \
    cp /usr/src/target/$(xx-info march)-unknown-linux-musl/release/clip-sync /

FROM rust:buster AS ui-builder
WORKDIR /usr/src/clip-sync
COPY . .
RUN rustup target add wasm32-unknown-unknown && cargo install dioxus-cli
RUN cd clip-sync-ui && dx build --release

FROM alpine
RUN apk add --update openssl bash
RUN mkdir /index
COPY --from=builder /clip-sync /app/clip-sync
COPY --from=builder /usr/src/config.toml /config/config.toml
COPY --from=ui-builder /usr/src/clip-sync/clip-sync-ui/dist /static-files
EXPOSE 3000
WORKDIR /
CMD ["/app/clip-sync", "--config", "/config/config.toml"]
