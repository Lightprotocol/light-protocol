Running `install.sh` is the first thing you should do after cloning this monorepo.

```bash
# Install all components (default)
./scripts/install.sh

# Install with all proving keys
./scripts/install.sh --full-keys

# Skip components (only use if you know what you are doing)
./scripts/install.sh --skip-components "redis,keys,go"
```

## Components

- `go` - Golang
- `rust` - Rust toolchain
- `node` - Node.js runtime
- `pnpm` - Package manager
- `solana` - Solana CLI tools
- `anchor` - Anchor
- `jq` - JSON processor
- `keys` - Gnark proving keys
- `dependencies` - all PNPM deps
- `redis` - Redis server

## CI Usage

```yaml
- name: Setup and build
  uses: ./.github/actions/setup-and-build
  with:
    skip-components: "redis"
```
