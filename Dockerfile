ARG SOLANA_REVISION=v1.9.12-testnet-with_trx_cap
ARG NEON_EVM_COMMIT=latest

FROM neonlabsorg/solana:${SOLANA_REVISION} AS solana
FROM neonlabsorg/evm_loader:${NEON_EVM_COMMIT} AS spl

FROM rust as builder
RUN apt update && apt install -y libudev-dev
COPY ./src /usr/src/faucet/src
COPY ./erc20 /usr/src/faucet/erc20
COPY ./Cargo.toml /usr/src/faucet
WORKDIR /usr/src/faucet
ARG REVISION
ENV FAUCET_REVISION=${REVISION}
RUN cargo build --release

FROM debian:11
RUN mkdir -p /opt/faucet
ADD internal/id.json /opt/faucet/
COPY --from=builder /usr/src/faucet/target/release/faucet /opt/faucet/

COPY --from=solana /opt/solana/bin/solana \
                /opt/solana/bin/solana-faucet \
                /opt/solana/bin/solana-keygen \
                /opt/solana/bin/solana-validator \
                /opt/solana/bin/solana-genesis \
                /cli/bin/
COPY --from=spl /opt/spl-token \
                /opt/create-test-accounts.sh \
                /opt/evm_loader-keypair.json /spl/bin/

CMD ["/opt/faucet/faucet"]
