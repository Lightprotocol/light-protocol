#!/usr/bin/env sh

set -eu

PREFIX="${PWD}/.local"
OS=`uname`
ARCH=`uname -m`

MAX_RETRIES=10

# Checks the latest release of the given GitHub repository.
latest_release() {
    OWNER="${1}"
    REPO="${2}"
    GITHUB="https://api.github.com"

    LATEST_RELEASE=$(curl --retry 5 --retry-delay 10 -s "${GITHUB}/repos/${OWNER}/${REPO}/releases/latest")

    # Extract the tag name
    TAG_NAME=$(echo "${LATEST_RELEASE}" | perl -ne 'print "${1}\n" if /"tag_name":\s*"([^"]*)"/' | head -1)

    echo "$TAG_NAME"
}

# Downloads a file from the given URL and places it in the given destination.
download_file() {
    url="${1}"
    dest_name="${2}"
    dest="${3}"

    echo "📥 Downloading ${dest_name}"

    for i in {0..$MAX_RETRIES}; do
        curl --fail --retry "${MAX_RETRIES}" --retry-delay 10 -L -o "${dest}/${dest_name}" "${url}"
        # Check if the file exists
        if [ -f "${dest}/${dest_name}" ]; then
            chmod +x "${dest}/${dest_name}"
            break
        else
            echo "Failed to download ${dest_name}. Retrying ($i of $MAX_RETRIES)..."
            if [ "$i" -eq $MAX_RETRIES ]; then
                echo "Failed to download ${dest_name} after $MAX_RETRIES attempts."
                exit 1
            fi
        fi
    done
}

# Downloads a tarball from the given URL and extracts it to the given
# destination.
download_and_extract() {
    archive_name="${1}"
    url="${2}"
    archive_type="${3}"
    dest="${4}"
    strip_components="${5:-0}"

    echo "📥 Downloading ${archive_name}"
    curl --retry 5 --retry-delay 10 -L "${url}" | tar "-x${archive_type}f" - --strip-components "${strip_components}" -C "${dest}"
}

# Downloads a file from the given GitHub repository and places it in the given
# destination.
download_file_github () {
    git_org="${1}"
    git_repo="${2}"
    git_release="${3}"
    src_name="${4}"
    dest_name="${5}"
    dest="${6}"

    download_file \
        "https://github.com/${git_org}/${git_repo}/releases/download/${git_release}/${src_name}" \
        "${dest_name}" \
        "${dest}"
}

# Downloads a tarball from the given GitHub repository and extracts it to the
# given destination.
download_and_extract_github () {
    git_org="${1}"
    git_repo="${2}"
    git_release="${3}"
    archive_name="${4}"
    archive_type="${5}"
    dest="${6}"
    strip_components="${7:-0}"

    download_and_extract \
        "${archive_name}" \
        "https://github.com/${git_org}/${git_repo}/releases/download/${git_release}/${archive_name}" \
        "${archive_type}" \
        "${dest}" \
        "${strip_components}"
}

# Check command line arguments for a specific flag
check_flag() {
    flag=$1
    shift  # This will shift the parameters, so $@ will hold the actual arguments
    for arg in "$@"
    do
        if [ "$arg" = "$flag" ]; then
            echo true
            return
        fi
    done
    echo false
}

GO_VERSION="1.21.7"
NODE_VERSION="20.9.0"
PNPM_VERSION="8.8.0"
SOLANA_VERSION="1.18.11"
ANCHOR_VERSION="anchor-v0.29.0"
JQ_VERSION="jq-1.7.1"
CIRCOM_VERSION=$(latest_release Lightprotocol circom)
MACRO_CIRCOM_VERSION=$(latest_release Lightprotocol macro-circom)
LIGHT_PROTOCOL_VERSION=$(latest_release Lightprotocol light-protocol)
PHOTON_VERSION="0.18.0"

case "${OS}" in
    "Darwin")
        case "${ARCH}" in
            "x86_64")
                ARCH_SUFFIX_GO="darwin-amd64"
                ARCH_SUFFIX_SOLANA="x86_64-apple-darwin"
                ARCH_SUFFIX_LP="macos-amd64"
                ARCH_SUFFIX_NODE="darwin-x64"
                ARCH_SUFFIX_PNPM="macos-x64"
                ARCH_SUFFIX_JQ="jq-osx-amd64"
                ;;
            "aarch64"|"arm64")
                ARCH_SUFFIX_GO="darwin-arm64"
                ARCH_SUFFIX_SOLANA="aarch64-apple-darwin"
                ARCH_SUFFIX_LP="macos-arm64"
                ARCH_SUFFIX_NODE="darwin-arm64"
                ARCH_SUFFIX_PNPM="macos-arm64"
                ARCH_SUFFIX_JQ="jq-macos-arm64"
                ;;
            "*")
                echo "Architecture ${ARCH} on operating system ${OS} is not supported."
                exit 1
                ;;
        esac
        ;;
    "Linux")
        case "${ARCH}" in
            "x86_64")
                ARCH_SUFFIX_GO="linux-amd64"
                ARCH_SUFFIX_SOLANA="x86_64-unknown-linux-gnu"
                ARCH_SUFFIX_LP="linux-amd64"
                ARCH_SUFFIX_NODE="linux-x64"
                ARCH_SUFFIX_PNPM="linuxstatic-x64"
                ARCH_SUFFIX_JQ="jq-linux-amd64"
                ;;
            "aarch64"|"arch64"|"arm64")
                ARCH_SUFFIX_GO="linux-arm64"
                ARCH_SUFFIX_SOLANA="aarch64-unknown-linux-gnu"
                ARCH_SUFFIX_LP="linux-arm64"
                ARCH_SUFFIX_NODE="linux-arm64"
                ARCH_SUFFIX_PNPM="linuxstatic-arm64"
                ARCH_SUFFIX_JQ="jq-linux-arm64"
                ;;
            "*")
                echo "Architecture ${ARCH} on operating system ${OS} is not supported."
                exit 1
                ;;
        esac
        ;;
    "*")
        echo "Operating system ${OS} is not supported."
        exit 1
        ;;
esac

echo "🔍 Detected system ${ARCH_SUFFIX_LP}"

echo "📁 Creating directory ${PREFIX}"
mkdir -p $PREFIX/bin/deps

echo "📥 Installing Go"
curl --fail --retry "${MAX_RETRIES}" --retry-delay 10 -L -o \
    /tmp/go.tar.gz \
    "https://go.dev/dl/go${GO_VERSION}.${ARCH_SUFFIX_GO}.tar.gz"
tar xpf /tmp/go.tar.gz -C "${PREFIX}"
rm -f /tmp/go.tar.gz
export PATH="${PREFIX}/go/bin:${PATH}"

echo "Downloading gnark keys"
# run the gnark-prover/scripts/download_keys.sh script
# this script will download the keys from IPFS and place them in the gnark-prover/circuits directory

# get root dir
ROOT_DIR="$(git rev-parse --show-toplevel)"
"$ROOT_DIR"/gnark-prover/scripts/download_keys.sh

echo "🦀 Installing Rust"
export RUSTUP_HOME="${PREFIX}/rustup"
export CARGO_HOME="${PREFIX}/cargo"
curl --retry 5 --retry-delay 10 --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    --no-modify-path # We want to control the PATH ourselves.
export PATH="${PREFIX}/cargo/bin:${PATH}"

rustup component add clippy
rustup component add rustfmt

cargo install cargo-expand wasm-pack
cargo install photon-indexer --version ${PHOTON_VERSION}
echo "📥 Downloading Node.js"
download_and_extract \
    "node-v${NODE_VERSION}-${ARCH_SUFFIX_NODE}.tar.gz" \
    "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-${ARCH_SUFFIX_NODE}.tar.gz" \
    z \
    "${PREFIX}" \
    1

NPM_DIR="${PREFIX}/npm-global"
mkdir -p "${NPM_DIR}"
export PATH="${PREFIX}/bin:${NPM_DIR}/bin:${PATH}"
export NPM_CONFIG_PREFIX="${NPM_DIR}"

echo "📥 Downloading pnpm"
download_file_github \
    pnpm \
    pnpm \
    "v${PNPM_VERSION}" \
    "pnpm-${ARCH_SUFFIX_PNPM}" \
    pnpm \
    "${PREFIX}/bin"

echo "📥 Downloading Solana toolchain"
download_and_extract_github \
    solana-labs \
    solana \
    "v${SOLANA_VERSION}" \
    "solana-release-${ARCH_SUFFIX_SOLANA}.tar.bz2" \
    j \
    "${PREFIX}/bin" \
    2

echo "📥 Downloading Anchor"
download_file_github \
    Lightprotocol \
    binaries \
    "${ANCHOR_VERSION}" \
    "anchor-${ARCH_SUFFIX_LP}" \
    anchor \
    "${PREFIX}/bin"

echo "📥 Downloading Jq"
download_file_github \
    jqlang \
    jq \
    "${JQ_VERSION}" \
    "${ARCH_SUFFIX_JQ}" \
    jq \
    "${PREFIX}/bin"

echo "📥 Downloading Circom"
download_file_github \
    Lightprotocol \
    circom \
    "${CIRCOM_VERSION}" \
    "circom-${ARCH_SUFFIX_LP}" \
    circom \
    "${PREFIX}/bin"

echo "📥 Downloading macro-circom"
download_file_github \
    Lightprotocol \
    macro-circom \
    "${MACRO_CIRCOM_VERSION}" \
    "macro-circom-${ARCH_SUFFIX_LP}" \
    macro-circom \
    "${PREFIX}/bin"

echo "📦 Installing pnpm dependencies"
pnpm install

echo "📦 Installing Playwright"
pnpm exec playwright install

echo "✨ Light Protocol development dependencies installed"