FROM --platform=$BUILDPLATFORM tonistiigi/xx AS xx
ARG TARGETARCH

FROM --platform=$BUILDPLATFORM rust:alpine AS builder
ENV OPENSSL_STATIC=yes
RUN apk add musl-dev alpine-sdk perl clang lld && rustup target add wasm32-unknown-unknown && cargo install dioxus-cli
ARG TARGETPLATFORM
WORKDIR /usr/src/clip-sync
COPY --from=xx / /
COPY . .
RUN xx-cargo build --release --no-default-features --features server-only && \
    cd clip-sync-ui && dx build --release

FROM --platform=$BUILDPLATFORM alpine
RUN apk add --update openssl bash
RUN mkdir /index
COPY --from=builder /usr/src/clip-sync/target/release/clip-sync /app/clip-sync
COPY --from=builder /usr/src/clip-sync/config-server.toml /config/config.toml
COPY --from=builder /usr/src/clip-sync/clip-sync-ui/dist /static-files
EXPOSE 3000
WORKDIR /
CMD ["/app/clip-sync", "--config", "/config/config.toml"]
