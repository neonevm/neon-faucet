#!/bin/bash

FAUCET_URL=$1

if [ -z "$FAUCET_URL" ]; then
  echo "Usage health_check_faucet.sh <faucet_url>"
  exit 1
fi

curl --location --request POST "$FAUCET_URL/request_ping" \
--header 'X-Ping-Header' \
--data-raw 'Healthcheck-ping'
