#!/bin/bash

# Run buildRelayer.sh
source ./scripts/buildDocker.sh

# Check if the Docker image was built successfully
if [[ "$(docker images -q relayer-app:latest 2> /dev/null)" == "" ]]; then
    echo "Test failed: Docker image was not built."


trap cleanup EXIT
    exit 1
else
    echo "Test passed: Docker image was built successfully."
    echo "Deleting builder instance..."
    docker buildx rm mybuilder
fi