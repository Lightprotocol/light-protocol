#!/bin/bash
set -e

PROJECT_ID=$(gcloud config get-value project)
REGION=europe-west1
REPO_NAME=light
IMAGE_NAME=prover-light
TAG=latest
FULL_IMAGE=$REGION-docker.pkg.dev/$PROJECT_ID/$REPO_NAME/$IMAGE_NAME:$TAG

# Authenticate podman with Google Artifact Registry
gcloud auth print-access-token | podman login -u oauth2accesstoken --password-stdin $REGION-docker.pkg.dev

# Build for amd64
podman build --platform linux/amd64 -t $IMAGE_NAME:$TAG -f Dockerfile.light .

# Tag and push
podman tag $IMAGE_NAME:$TAG $FULL_IMAGE
podman push $FULL_IMAGE
