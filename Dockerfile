FROM rust as builder
RUN apt update && apt install libudev-dev
RUN git clone https://github.com/neonlabsorg/neon-faucet.git /usr/src/faucet
WORKDIR /usr/src/faucet
RUN git checkout faucet-parallel-clients
RUN FAUCET_REVISION=$(git rev-parse HEAD) cargo build --release

FROM debian:11
RUN mkdir -p /opt/faucet
ADD internal/id.json /opt/faucet/
COPY --from=builder /usr/src/faucet/target/release/faucet /opt/faucet/
CMD ["/opt/faucet/faucet"]

