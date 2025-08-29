Scripts to fetch Solana accounts and save them as JSON files. Also supports LUTs.

## Building

```bash
cd fetch-accounts
cargo build --release
```

## Usage

### 1. Test Env Fetcher (`fetch_test`)

This script uses LightProgramTest to fetch accounts from test state trees:

```bash
cargo run --bin fetch_test
```

### 2. RPC Fetcher (`fetch_rpc`)

Fetch specific accounts from any Solana network:

```bash
# Fetch from mainnet
NETWORK=mainnet cargo run --bin fetch_rpc <pubkey1> <pubkey2> ...

# Fetch from devnet
NETWORK=devnet cargo run --bin fetch_rpc <pubkey1> <pubkey2> ...

# Fetch from local validator
cargo run --bin fetch_rpc <pubkey1> <pubkey2> ...

# Use custom RPC endpoint
RPC_URL=https://your-rpc.com cargo run --bin fetch_rpc <pubkey1> <pubkey2> ...
```

#### Network Options

- `NETWORK=mainnet` - Solana Mainnet Beta
- `NETWORK=devnet` - Solana Devnet
- `NETWORK=testnet` - Solana Testnet
- `NETWORK=local` - Local validator (default)
- `RPC_URL=<url>` - Custom RPC endpoint

#### Regular Account Examples

```bash
# Fetch System Program and Token Program from mainnet
NETWORK=mainnet cargo run --bin fetch_rpc \
  11111111111111111111111111111111 \
  TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA

# Fetch a specific account from devnet
NETWORK=devnet cargo run --bin fetch_rpc So11111111111111111111111111111111111111112
```

### 3. Address Lookup Table

Get LUTs for localnet. It sets `last_extended_slot = 0` so it works reliably with your
test-ledger.

Upload the LUT to your test-ledger via: `--account LUT_ADDRESS_BASE58 ./dir/to/lut.json`

```bash
# Process a lookup table from mainnet (sets last_extended_slot to 0)
IS_LUT=true NETWORK=mainnet cargo run --bin fetch_rpc <lut_pubkey>

# Process multiple lookup tables
IS_LUT=true NETWORK=mainnet cargo run --bin fetch_rpc <lut1> <lut2> <lut3>

# Use with custom RPC
IS_LUT=true RPC_URL=https://api.mainnet-beta.solana.com cargo run --bin fetch_rpc <lut_pubkey>
```
