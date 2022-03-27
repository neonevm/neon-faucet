FROM rust as builder
RUN apt update && apt install -y libudev-dev
COPY ./src /usr/src/faucet/src
COPY ./Cargo.toml /usr/src/faucet
WORKDIR /usr/src/faucet
ARG REVISION
ENV FAUCET_REVISION=${REVISION}
RUN cargo build --release

FROM debian:11
RUN mkdir -p /opt/faucet
ADD internal/id.json /opt/faucet/
COPY --from=builder /usr/src/faucet/target/release/faucet /opt/faucet/
CMD ["/opt/faucet/faucet"]
