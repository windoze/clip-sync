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
RUN cargo build --package clip-sync-server --release --target=$(xx-info march)-unknown-linux-musl && \
    cp /usr/src/target/$(xx-info march)-unknown-linux-musl/release/clip-sync-server /clip-sync

FROM node:alpine AS ui-builder
WORKDIR /usr/src/clip-sync
COPY . .
RUN cd clip-sync-ui && npm install && npm run build

FROM alpine
RUN apk add --update openssl bash
RUN mkdir /index /images
COPY --from=builder /clip-sync /app/clip-sync
COPY --from=builder /usr/src/config-server.toml /config/config.toml
COPY --from=ui-builder /usr/src/clip-sync/clip-sync-ui/dist /static-files
EXPOSE 3000
WORKDIR /
CMD ["/app/clip-sync", "--config", "/config/config.toml", "-vv"]
