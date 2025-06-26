#!/usr/bin/env bash

# Exit on any error
set -e

# Function to handle errors
handle_error() {
    echo "❌ Error occurred on line $1"
    exit 1
}

# Set trap to catch errors
trap 'handle_error $LINENO' ERR

# Check for required tools
echo "🔍 Checking required dependencies..."
if ! command -v pnpm >/dev/null 2>&1; then
    echo "❌ pnpm is not installed. Run ./scripts/install.sh to install dependencies."
    exit 1
fi

if ! command -v npx >/dev/null 2>&1; then
    echo "❌ npx is not installed. Run ./scripts/install.sh to install dependencies."
    exit 1
fi

echo "📦 Installing project dependencies..."
pnpm install || { echo "❌ Failed to install dependencies. Check your internet connection and access rights."; exit 1; }

echo "🔧 Checking for required files..."
if [ ! -f target/deploy/spl_noop.so ]; then
    echo "📄 Copying spl_noop.so..."
    mkdir -p target/deploy && cp third-party/solana-program-library/spl_noop.so target/deploy
fi

echo "🚀 Starting build process for all packages..."
npx nx run-many --target=build --all || { echo "❌ Error during package build."; exit 1; }

echo "✅ Build process completed successfully."
