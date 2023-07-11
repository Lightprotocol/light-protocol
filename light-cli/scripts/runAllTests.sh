#!/bin/bash

set -eux

"yarn test-shield && \
yarn test-balance && \
yarn test-config && \
yarn test-shield:sol && \
yarn test-shield:spl && \
yarn test-unshield && \
yarn test-unshield:sol && \
yarn test-unshield:spl && \
yarn test-transfer && \
yarn test-accept_utxos"