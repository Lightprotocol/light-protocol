#!/usr/bin/env sh

set -eux

# Locate the light-anchor executable
LIGHT_ANCHOR_PATH=$(which light-anchor)

# Check if the light-anchor executable was found
if [ -z "$LIGHT_ANCHOR_PATH" ]; then
    echo "Error: light-anchor executable not found in PATH"
    exit 1
fi

# Copy the light-anchor executable
cp "$LIGHT_ANCHOR_PATH" .

# Build the Docker image
docker build -t relayer-app:latest .

# Remove the copied executable
rm light-anchor

# Tag the Docker image
docker tag relayer-app:latest registry.digitalocean.com/v3-relayer/relayer-app:latest

# Login to DigitalOcean Docker registry
doctl registry login

# Push the Docker image to the registry
docker push registry.digitalocean.com/v3-relayer/relayer-app:latest