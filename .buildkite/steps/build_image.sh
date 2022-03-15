#!/bin/bash
set -euo pipefail

FAUCET_REVISION=$(git rev-parse HEAD)

docker build -t neonlabsorg/neon-faucet:$FAUCET_REVISION .

docker images
