#!/bin/bash
set -euo pipefail

#FAUCET_REVISION=$(git rev-parse HEAD)

docker login -u=${DHUBU} -p=${DHUBP}
#docker push neonlabsorg/faucet:$FAUCET_REVISION
docker push neonlabsorg/faucet:latest
