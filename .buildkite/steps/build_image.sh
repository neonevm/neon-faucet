#!/bin/bash
set -euo pipefail

echo "Neon Faucet revision = ${BUILDKITE_COMMIT}"

docker build -t neonlabsorg/faucet:${BUILDKITE_COMMIT} .

docker images
