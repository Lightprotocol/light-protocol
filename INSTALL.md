# Light Protocol Installation

Running `install.sh` is the first thing you should do after cloning this monorepo.

```bash
# Install all components (default)
./scripts/install.sh

# Install with all proving keys
./scripts/install.sh --full-keys

# Skip components (only use if you know what you are doing)
./scripts/install.sh --skip-components "redis,keys,go"
```

## System Requirements

- **Linux/macOS**: Ubuntu 20.04+/macOS 12+
- **RAM**: Minimum 8 GB, recommended 16 GB
- **Disk space**: Minimum 10 GB free space
- **Dependencies**: build-essential, curl, git

### Installing Dependencies

#### Ubuntu/Debian
```bash
sudo apt update
sudo apt install build-essential curl git autoconf automake libtool zlib1g-dev pkg-config libssl-dev
```

#### macOS
```bash
xcode-select --install
brew install automake libtool pkg-config openssl
```

## Components

- `go` - Golang (for working with network components)
- `rust` - Rust toolchain (base programming language)
- `node` - Node.js runtime (for JavaScript/TypeScript)
- `pnpm` - Package manager (JavaScript dependency management)
- `solana` - Solana CLI tools (tools for working with Solana)
- `anchor` - Anchor (framework for Solana development)
- `jq` - JSON processor (JSON processing in scripts)
- `keys` - Gnark proving keys (keys for ZK proofs)
- `dependencies` - all PNPM deps (JavaScript project dependencies)
- `redis` - Redis server (for caching and asynchronous task processing)

## Common Issues and Solutions

### General Issues

- **Access errors**: Make sure you have write permissions to the project directory
- **Disk space**: Check available disk space (especially when using `--full-keys`)
- **Network errors**: Check your internet connection and package server availability

### Specific Issues

- **Redis**: If you have problems installing Redis, you can install it separately through your package manager:
  - Ubuntu: `sudo apt install redis-server`
  - macOS: `brew install redis`

- **Keys**: If key download fails, try running:
  ```bash
  ./prover/server/scripts/download_keys.sh
  ```

- **Node.js**: If you already have Node.js installed but the script doesn't recognize it, try skipping this component:
  ```bash
  ./scripts/install.sh --skip-components "node"
  ```

## CI Usage

```yaml
- name: Setup and build
  uses: ./.github/actions/setup-and-build
  with:
    skip-components: "redis"
```

## After Installation

After successful installation, run:

```bash
# Verify successful installation
./scripts/devenv.sh

# Build the project
./scripts/build.sh
```
