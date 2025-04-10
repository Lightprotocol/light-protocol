# Component-Based Installation

The `install.sh` script now supports selective component installation to reduce CI/CD times.

## Usage

```bash
# Install all components (default)
./scripts/install.sh

# Install specific components only
./scripts/install.sh --components "node,pnpm,dependencies"

# Install with full keys
./scripts/install.sh --full-keys --components "rust,solana,anchor"
```

## Available Components

- `go` - Golang
- `rust` - Rust toolchain
- `node` - Node.js runtime
- `pnpm` - Package manager
- `solana` - Solana CLI tools
- `anchor` - Anchor
- `jq` - JSON processor
- `keys` - Gnark proving keys
- `dependencies` - all PNPM deps
- `redis` - Redis server (not needed for some tests)

## GitHub Actions Usage

In workflow files, specify components via the `setup-and-build` action:

```yaml
- name: Setup and build
  uses: ./.github/actions/setup-and-build
  with:
    components: "node,pnpm,solana,anchor,jq,keys,dependencies"
```
