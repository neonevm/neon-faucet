#!/bin/bash
set -euo pipefail

#export FAUCET_REVISION=$(git rev-parse HEAD)

#docker build -t neonlabsorg/faucet:$FAUCET_REVISION .
docker build -t neonlabsorg/faucet:latest .

docker images
