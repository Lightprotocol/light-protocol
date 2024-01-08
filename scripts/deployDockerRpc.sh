#!/bin/bash

bash $(dirname $0)/buildDockerRpc.sh

cleanupDeploy() {
    echo "Deleting builder instance..."
    docker buildx rm mybuilder
}

trap cleanupDeploy EXIT

doctl registry login
docker push registry.digitalocean.com/v3-rpc/rpc-app:latest

