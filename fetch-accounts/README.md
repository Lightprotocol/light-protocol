# Account Fetcher Scripts

Two standalone Rust scripts to fetch Solana accounts and save them as JSON files, with special support for Address Lookup Tables.

## Building

```bash
cd fetch-accounts
cargo build --release
```

## Usage

### 1. Test Environment Fetcher (`fetch_test`)

This script uses the Light Protocol test environment to fetch accounts from test state trees:

```bash
cargo run --bin fetch_test
```

### 2. RPC Fetcher (`fetch_rpc`) - **Recommended**

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

### 3. Address Lookup Table Processing

Set `IS_LUT=true` to decode, modify, and re-encode Address Lookup Tables:

```bash
# Process a lookup table from mainnet (sets last_extended_slot to 0)
IS_LUT=true NETWORK=mainnet cargo run --bin fetch_rpc <lut_pubkey>

# Process multiple lookup tables
IS_LUT=true NETWORK=mainnet cargo run --bin fetch_rpc <lut1> <lut2> <lut3>

# Use with custom RPC
IS_LUT=true RPC_URL=https://api.mainnet-beta.solana.com cargo run --bin fetch_rpc <lut_pubkey>
```

#### What LUT Processing Does

When `IS_LUT=true` is set, the script will:

1. **Fetch** the Address Lookup Table account data
2. **Decode** the binary structure according to Solana's LUT format
3. **Analyze** and display LUT metadata:
   - Discriminator (should be 1)
   - Deactivation slot
   - Current last_extended_slot value
   - Authority (if any)
   - Number of stored addresses
4. **Modify** the `last_extended_slot` field to `0`
5. **Re-encode** the data in the same binary format
6. **Save** as `modified_lut_<pubkey>.json` with base64-encoded modified data

This is useful for testing scenarios where you need a lookup table with `last_extended_slot = 0`.

## Output Format

### Regular Accounts

Saved as `account_<pubkey>.json`:

```json
{
  "pubkey": "...",
  "account": {
    "lamports": 1000000,
    "data": ["base64_encoded_data", "base64"],
    "owner": "...",
    "executable": false,
    "rentEpoch": 0,
    "space": 165
  }
}
```

### Modified Lookup Tables

Saved as `modified_lut_<pubkey>.json` with the same structure but modified binary data.

## Notes

- The RPC fetcher is the recommended tool for most use cases
- JSON files are saved in the current working directory
- Invalid pubkeys will show an error but won't stop the process
- LUT processing requires the account to be a valid Address Lookup Table
- The binary modification preserves all data except `last_extended_slot`
