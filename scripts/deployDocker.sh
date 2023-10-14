#!/bin/bash

# builds relayer as docker image (buildDocker.sh) and deploys to digitalocean
source $(dirname $0)/buildDocker.sh

cleanup() {
    echo "Deleting builder instance..."
    docker buildx rm mybuilder
}

trap cleanup EXIT

doctl registry login
docker push registry.digitalocean.com/v3-relayer/relayer-app:latest

