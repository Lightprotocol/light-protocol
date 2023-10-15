#!/bin/bash

source $(dirname $0)/buildDockerRelayer.sh

cleanup() {
    echo "Deleting builder instance..."
    docker buildx rm mybuilder
}

trap cleanup EXIT

doctl registry login
docker push registry.digitalocean.com/v3-relayer/relayer-app:latest

