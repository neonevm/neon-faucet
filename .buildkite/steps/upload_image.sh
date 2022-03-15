#!/bin/bash
set -euo pipefail

FAUCET_REVISION=$(git rev-parse HEAD)


docker login -u=${DHUBU} -p=${DHUBP}
docker push neonlabsorg/neon-faucet:$FAUCET_REVISION .
