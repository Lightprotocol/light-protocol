# Docker Setup for Light Prover

This document describes the Docker setup for the Light Protocol prover server, including build processes and deployment workflows.

## Available Docker Images

### 1. Full Prover Image (`Dockerfile`)

The main Docker image that includes all necessary proving keys for production use.

**Features:**
- Contains pre-downloaded proving keys (mainnet, inclusion, non-inclusion, combined)
- Ready to use for proof generation
- Larger image size due to embedded keys
- Built via `prover-docker-release.yml` workflow

**Usage:**
```bash
docker run ghcr.io/lightprotocol/light-protocol/light-prover:latest start --run-mode rpc --keys-dir /proving-keys/
```

### 2. Light Prover Image (`Dockerfile.light`)

A lightweight image without proving keys, suitable for development or custom key management.

**Features:**
- No embedded proving keys
- Smaller image size
- Requires external key management
- Built via `prover-docker-light-release.yml` workflow

**Usage:**
```bash
# Mount your own keys directory
docker run -v /path/to/your/keys:/proving-keys ghcr.io/lightprotocol/light-protocol/light-prover-light:latest start --keys-dir /proving-keys/

# Or run without keys for development
docker run ghcr.io/lightprotocol/light-protocol/light-prover-light:latest start
```

## Key Management Scripts

### `scripts/download_keys_docker.sh`

Specialized script that downloads only the proving keys needed for the Docker build:

- `mainnet_inclusion_26_*` keys
- `inclusion_32_*` keys
- `non-inclusion_26_*` and `non-inclusion_40_*` keys
- `combined_26_*` and `combined_32_40_*` keys

This is more efficient than the full `download_keys.sh light` script as it excludes:
- `append-with-proofs_32_*` keys
- `update_32_*` keys
- `address-append_40_*` keys

### `scripts/download_keys.sh`

Original script with two modes:
- `light`: Downloads keys including batch operations (less efficient for Docker)
- `full`: Downloads all available keys

## GitHub Workflows

### `prover-docker-release.yml`

Builds and publishes the full prover image with embedded keys.

**Triggers:**
- Push to tags matching `light-prover*`
- Manual workflow dispatch

**Process:**
1. Downloads proving keys using `download_keys_docker.sh`
2. Builds Docker image with `Dockerfile`
3. Pushes to GitHub Container Registry
4. Cleans up downloaded keys

### `prover-docker-light-release.yml`

Builds and publishes the lightweight prover image without keys.

**Triggers:**
- Push to tags matching `light-prover*`
- Manual workflow dispatch

**Process:**
1. Builds Docker image with `Dockerfile.light`
2. Pushes to GitHub Container Registry

## Local Development

### Building Images Locally

For the full image:
```bash
cd prover/server
./scripts/download_keys_docker.sh
docker build -t light-prover .
```

For the light image:
```bash
cd prover/server
docker build -f Dockerfile.light -t light-prover-light .
```

### Testing Images

Test the full image:
```bash
docker run --rm light-prover start --run-mode rpc --keys-dir /proving-keys/
```

Test the light image:
```bash
docker run --rm light-prover-light start
```

## Image Registry

Both images are published to GitHub Container Registry:

- Full image: `ghcr.io/lightprotocol/light-protocol/light-prover`
- Light image: `ghcr.io/lightprotocol/light-protocol/light-prover-light`

Tags follow the pattern:
- `latest`: Latest release from main branch
- `<tag-name>`: Specific version tags (e.g., `light-prover-v1.0.0`)
