#!/usr/bin/env sh

set -eux

# Build the Docker image
docker build -t relayer-app:latest .

# Tag the Docker image
docker tag relayer-app:latest registry.digitalocean.com/v3-relayer/relayer-app:latest

# Login to DigitalOcean Docker registry
doctl registry login

# Push the Docker image to the registry
docker push registry.digitalocean.com/v3-relayer/relayer-app:latest