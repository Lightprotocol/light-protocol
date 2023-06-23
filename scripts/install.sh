#!/bin/bash

set -eu

PREFIX=$(pwd)/.local
ARCH=$(uname -m)

# Checks the latest release of the given GitHub repository.
function latest_release() {
    local OWNER="$1"
    local REPO="$2"
    local GITHUB="https://api.github.com"

    local LATEST_RELEASE=$(curl -s $GITHUB/repos/$OWNER/$REPO/releases/latest)

    # Extract the tag name
    local TAG_NAME=$(echo "$LATEST_RELEASE" | perl -ne 'print "$1\n" if /"tag_name":\s*"([^"]*)"/' | head -1)

    echo "$TAG_NAME"
}

# Downloads a file from the given URL and places it in the given destination.
function download_file() {
    local url=$1
    local dest_name=$2
    local dest=$3

    echo "📥 Downloading ${dest_name}"
    curl -L -o ${dest}/${dest_name} ${url}
    chmod +x ${dest}/${dest_name}
}

# Downloads a tarball from the given URL and extracts it to the given
# destination.
function download_and_extract() {
    local archive_name=$1
    local url=$2
    local dest=$3
    local strip_components=${4:-0}

    echo "📥 Downloading ${archive_name}"
    curl -L ${url} | tar -zxf - --strip-components ${strip_components} -C ${dest}
}

# Downloads a file from the given GitHub repository and places it in the given
# destination.
function download_file_github () {
    local git_org=$1
    local git_repo=$2
    local git_release=$3
    local src_name=$4
    local dest_name=$5
    local dest=$6

    download_file \
        https://github.com/${git_org}/${git_repo}/releases/download/${git_release}/${src_name} \
        ${dest_name} \
        ${dest}
}

# Downloads a tarball from the given GitHub repository and extracts it to the
# given destination.
function download_and_extract_github () {
    local git_org=$1
    local git_repo=$2
    local git_release=$3
    local archive_name=$4
    local dest=$5
    local strip_components=${6:-0}

    download_and_extract \
        ${archive_name} \
        https://github.com/${git_org}/${git_repo}/releases/download/${git_release}/${archive_name} \
        ${dest} \
        ${strip_components}
}

NODE_VERSION="16.20.1"
SOLANA_VERSION="1.16.1"
ANCHOR_VERSION=$(latest_release Lightprotocol anchor)
CIRCOM_VERSION=$(latest_release Lightprotocol circom)
MACRO_CIRCOM_VERSION=$(latest_release Lightprotocol macro-circom)
LIGHT_PROTOCOL_VERSION=$(latest_release Lightprotocol light-protocol)

if ! rustup toolchain list 2>/dev/null | grep -q "nightly"; then
    echo "Rust nightly is not installed!"
    echo "Please install https://rustup.rs/ and then install the nightly toolchain with:"
    echo "    rustup toolchain install nightly" 
fi

case $ARCH in
    "x86_64")
        ARCH_SUFFIX_SOLANA="x86_64-unknown-linux-gnu"
        ARCH_SUFFIX_LP="linux-amd64"
        ARCH_SUFFIX_NODE="linux-x64"
        ;;
    "aarch64")
        ARCH_SUFFIX_SOLANA="aarch64-unknown-linux-gnu"
        ARCH_SUFFIX_LP="linux-arm64"
        ARCH_SUFFIX_NODE="linux-arm64"
        ;;
    "arm64")
        ARCH_SUFFIX_SOLANA="aarch64-apple-darwin"
        ARCH_SUFFIX_LP="macos-arm64"
        ARCH_SUFFIX_NODE="darwin-arm64"
        ;;
    *)
        echo "Architecture $ARCH is not supported."
        exit 1
        ;;
esac

echo "🔍 Detected system $ARCH_SUFFIX_LP"

echo "📁 Creating directory $PREFIX"
mkdir -p $PREFIX/bin/deps

echo "🦀 Installing Rust"
export RUSTUP_HOME=$PREFIX/rustup
export CARGO_HOME=$PREFIX/cargo
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    --no-modify-path # We want to control the PATH ourselves.
source $CARGO_HOME/env

echo "📥 Downloading Node.js"
download_and_extract \
    node-v${NODE_VERSION}-${ARCH_SUFFIX_NODE}.tar.gz \
    https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-${ARCH_SUFFIX_NODE}.tar.gz \
    ${PREFIX} \
    1

NPM_DIR=$PREFIX/npm-global
mkdir -p $NPM_DIR
export PATH=$PREFIX/bin:$NPM_DIR/bin:$PATH
export NPM_CONFIG_PREFIX=$NPM_DIR

echo "📦 Installing yarn"
npm install -g yarn

echo "📦 Installing TypeScript"
yarn global add typescript

echo "📥 Downloading Solana toolchain"
download_and_extract_github \
    solana-labs \
    solana \
    v${SOLANA_VERSION} \
    solana-release-${ARCH_SUFFIX_SOLANA}.tar.bz2 \
    ${PREFIX}/bin \
    2

echo "📥 Downloading Light Anchor"
download_file_github \
    Lightprotocol \
    anchor \
    ${ANCHOR_VERSION} \
    light-anchor-${ARCH_SUFFIX_LP} \
    light-anchor \
    ${PREFIX}/bin

echo "📥 Downloading Circom"
download_file_github \
    Lightprotocol \
    circom \
    ${CIRCOM_VERSION} \
    circom-${ARCH_SUFFIX_LP} \
    circom \
    ${PREFIX}/bin

echo "📥 Downloading macro-circom"
download_file_github \
    Lightprotocol \
    macro-circom \
    ${MACRO_CIRCOM_VERSION} \
    macro-circom-${ARCH_SUFFIX_LP} \
    macro-circom \
    ${PREFIX}/bin

echo "✨ Light Protocol development dependencies installed"
