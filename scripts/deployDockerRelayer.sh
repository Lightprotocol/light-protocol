#!/bin/bash

bash $(dirname $0)/buildDockerRelayer.sh

cleanupDeploy() {
    echo "Deleting builder instance..."
    docker buildx rm mybuilder
}

trap cleanupDeploy EXIT

doctl registry login
docker push registry.digitalocean.com/v3-relayer/relayer-app:latest

