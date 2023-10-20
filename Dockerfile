ARG NEON_EVM_COMMIT=latest

FROM neonlabsorg/evm_loader:${NEON_EVM_COMMIT} AS spl

FROM rust as builder
RUN apt update && apt install -y libudev-dev
COPY ./src /usr/src/faucet/src
COPY ./rust-web3 /usr/src/faucet/rust-web3
COPY ./erc20 /usr/src/faucet/erc20
COPY ./Cargo.toml /usr/src/faucet
WORKDIR /usr/src/faucet
ARG REVISION
ENV FAUCET_REVISION=${REVISION}
RUN cargo build --release

FROM debian:12
RUN apt update && apt install -y ca-certificates curl
RUN mkdir -p /opt/faucet
WORKDIR /opt/faucet
ADD internal/id.json /opt/faucet/
RUN mkdir -p /root/.config/solana && ln -s /opt/faucet/id.json /root/.config/solana/id.json
ADD *.sh /opt/faucet/
ADD faucet.conf /opt/faucet/
COPY --from=builder /usr/src/faucet/target/release/faucet /opt/faucet/
RUN ln -s /opt/faucet/faucet /usr/local/bin/

COPY --from=spl /opt/solana/bin/solana \
		/opt/solana/bin/solana-faucet \
		/opt/solana/bin/solana-keygen \
		/opt/solana/bin/solana-validator \
		/opt/solana/bin/solana-genesis \
		/usr/local/bin/
COPY --from=spl /opt/spl-token \
		/opt/neon-cli \
		/opt/create-test-accounts.sh \
		/opt/evm_loader-keypair.json \
		/spl/bin/

COPY --from=spl /opt/keys/neon_token_keypair.json /opt/faucet/owner-keypair.json

COPY --from=spl /opt/spl-token \
		/usr/local/bin/

CMD ["/opt/faucet/faucet"]
