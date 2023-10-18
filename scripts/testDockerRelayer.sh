#!/bin/bash

source ./scripts/buildDockerRelayer.sh

if [[ "$(docker images -q relayer-app:latest 2> /dev/null)" == "" ]]; then
    echo "Test failed: Docker image was not built."

trap cleanup EXIT
    exit 1
else
    echo "Test passed: Docker image was built successfully."
    echo "Deleting builder instance..."
    docker buildx rm mybuilder
fi