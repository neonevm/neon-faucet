#!/bin/bash
set -euo pipefail

echo "Neon Faucet revision = ${BUILDKITE_COMMIT}"

#export FAUCET_REVISION=$(git rev-parse HEAD)

#docker build -t neonlabsorg/faucet:$FAUCET_REVISION .
docker build --build-arg REVISION=${BUILDKITE_COMMIT} --no-cache=true -t neonlabsorg/faucet:latest .

docker images
