#!/bin/bash

# Script to debug CI issues locally by replicating GitHub Actions environment
# This matches the setup in .github/workflows/cli-v1.yml and cli-v2.yml

set -e

echo "=== Setting up local CI debugging environment ==="

# Check if Redis is running
if ! command -v redis-cli &> /dev/null; then
    echo "❌ Redis is not installed. Please install Redis first."
    echo "   On macOS: brew install redis"
    echo "   On Ubuntu: sudo apt-get install redis-server"
    exit 1
fi

# Start Redis if not running
if ! redis-cli ping &> /dev/null; then
    echo "Starting Redis..."
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        brew services start redis
    else
        # Linux
        sudo systemctl start redis || redis-server --daemonize yes
    fi
    
    # Wait for Redis to be ready
    for i in {1..10}; do
        if redis-cli ping &> /dev/null; then
            echo "✅ Redis is running"
            break
        fi
        echo "Waiting for Redis to start..."
        sleep 1
    done
fi

# Set environment variables to match CI
export REDIS_URL="redis://localhost:6379"
export LIGHT_PROTOCOL_VERSION="${1:-V1}"  # Default to V1, can pass V2 as argument

echo "Environment variables set:"
echo "  REDIS_URL=$REDIS_URL"
echo "  LIGHT_PROTOCOL_VERSION=$LIGHT_PROTOCOL_VERSION"

# Clean up any existing test artifacts
echo "Cleaning up test artifacts..."
rm -rf test-ledger/
rm -rf node_modules/.cache/nx

# Source devenv
echo "Sourcing devenv..."
source ../scripts/devenv.sh

# Build dependencies based on protocol version
echo "Building dependencies for $LIGHT_PROTOCOL_VERSION..."
if [ "$LIGHT_PROTOCOL_VERSION" = "V1" ]; then
    echo "Building stateless.js with V1..."
    (cd ../js/stateless.js && pnpm build:v1)
    
    echo "Building compressed-token with V1..."
    (cd ../js/compressed-token && pnpm build:v1)
else
    echo "Building stateless.js with V2..."
    (cd ../js/stateless.js && pnpm build:v2)
    
    echo "Building compressed-token with V2..."
    (cd ../js/compressed-token && pnpm build:v2)
fi

# Build CLI
echo "Building CLI..."
npx nx build @lightprotocol/zk-compression-cli --skip-nx-cache

# Run tests
echo "Running CLI tests..."
echo "==================================="
npx nx test @lightprotocol/zk-compression-cli

# Check exit code
if [ $? -eq 0 ]; then
    echo "✅ Tests passed!"
else
    echo "❌ Tests failed!"
    
    # Display prover logs on failure (matching CI behavior)
    echo "=== Displaying prover logs ==="
    find test-ledger -name "*prover*.log" -type f -exec echo "=== Contents of {} ===" \; -exec cat {} \; -exec echo "=== End of {} ===" \; || echo "No prover logs found"
    
    exit 1
fi 