# ZK Compression CLI

CLI to interact with compressed accounts and Light Tokens on Solana.

## Requirements

- Ensure you have Node >= v20.9.0 installed on your machine.

- You will need a valid Solana filesystem wallet set up at `~/.config/solana/id.json`.
  If you don't have one yet, visit the [Solana documentation](https://docs.solanalabs.com/cli/wallets/file-system) for details.
  The CLI will use this wallet as the default fee payer and mint authority.

## Installation

### Using npm

```bash
npm install -g @lightprotocol/zk-compression-cli
```

### Building from source

If you prefer to build the CLI from source, follow the steps below to install
the necessary prerequisites.

**1. Activate the Development Environment**

Ensure you are at the root of the monorepo.

```bash
. ./scripts/devenv
```

**2. Install and build the monorepo from source. This also builds the CLI.**

```bash
./scripts/install.sh
```

```bash
./scripts/build.sh
```

**3. Make your CLI available globally**

```bash
pnpm link --global
```

```bash
# Verify the CLI was correctly installed
which light
```

## Usage

**1. Once globally installed, start the Light test validator**

```bash
light test-validator
```

This starts a Solana test-validator with the Light system programs and accounts, a prover server, and the Photon indexer as background processes against a clean ledger.

```bash
# Pass --skip-indexer to start without the indexer
light test-validator --skip-indexer

# Pass --skip-prover to start without the prover
light test-validator --skip-prover

```

> **Note:** The CLI currently runs the photon indexer and light-prover as background processes at port: `8784` and `3001` respectively.

**2. Ensure you have sufficient localnet funds**

```bash
# Airdrop 1 SOL
solana airdrop 1

# Print your address
solana address

# Print your balance
solana balance
```

Now you're all set up to run CLI commands :)

#### Using Devnet

By default, the CLI interacts with localnet. You can view the current config by running:

```bash
light config --get
```

To switch to Devnet, point the URLs to an RPC supporting ZK Compression. For example, run:

```bash
  light config --indexerUrl "https://devnet.helius-rpc.com/?api-key=<api-key>" \
    --proverUrl "https://devnet.helius-rpc.com/?api-key=<api-key>" \
    --solanaRpcUrl "https://devnet.helius-rpc.com/?api-key=<api-key>"
```

Also adjust your solana config:

```bash
# Set config
solana config set --url "https://devnet.helius-rpc.com/?api-key=<api-key>"

# Airdrop 1 SOL
solana airdrop 1

# Print your address
solana address
```

### Commands

#### Create a Light Token mint

```bash
light create-mint
```

```
USAGE
  $ light create-mint [--mint-keypair <value>] [--mint-authority <value>]
    [--mint-decimals <value>]

FLAGS
  --mint-authority=<value>  Path to the mint authority keypair file.
                            Defaults to default local Solana wallet file
                            path.
  --mint-decimals=<value>   Number of base 10 digits to the right
                            of the decimal place [default: 9].
  --mint-keypair=<value>    Path to a mint keypair file. Defaults to a
                            random keypair.
```

#### Create a token account

```bash
light create-token-account YOUR_MINT_ADDRESS
```

```
USAGE
  $ light create-token-account MINT [--owner <value>]

ARGUMENTS
  MINT  Base58 encoded mint address.

FLAGS
  --owner=<value>  Owner of the token account. Defaults to the fee
                   payer's public key.
```

#### Mint Light Tokens to a Solana wallet

```bash
light mint-to --mint "YOUR_MINT_ADDRESS" --to "YOUR_WALLET_ADDRESS" --amount 4200000000
```

```
USAGE
  $ light mint-to --mint <value> --to <value> --amount <value>
    [--mint-authority <value>]

FLAGS
  --amount=<value>          (required) Amount to mint.
  --mint=<value>            (required) Mint address.
  --mint-authority=<value>  File path of the mint authority keypair.
                            Defaults to local Solana wallet.
  --to=<value>              (required) Recipient address.
```

#### Transfer Light Tokens from one wallet to another

```bash
light transfer --mint "YOUR_MINT_ADDRESS" --to "RECIPIENT_WALLET_ADDRESS" --amount 4200000000
```

```
USAGE
  $ light transfer --mint <value> --to <value> --amount <value>
    [--fee-payer <value>]

FLAGS
  --amount=<value>     (required) Amount to send.
  --fee-payer=<value>  Fee payer account. Defaults to the client
                       keypair.
  --mint=<value>       (required) Mint to transfer
  --to=<value>         (required) Recipient address

```

#### Wrap SOL into compressed account

```bash
light wrap-sol --amount 1000 --to "YOUR_WALLET_ADDRESS_BASE58"
```

```
USAGE
  $ light wrap-sol --to <value> --amount <value>

FLAGS
  --amount=<value>  (required) Amount to wrap in lamports.
  --to=<value>      (required) Specify the recipient address.
```

#### Unwrap SOL from compressed account

```bash
light unwrap-sol --amount 42 --to "YOUR_WALLET_ADDRESS_BASE58"
```

```
USAGE
  $ light unwrap-sol --to <value> --amount <value>

FLAGS
  --amount=<value>  (required) Amount to unwrap in lamports.
  --to=<value>      (required) Specify the recipient address.
```

#### Get token balance

```bash
light token-balance --mint "YOUR_MINT_ADDRESS" --owner "OWNER_ADDRESS"
```

```
USAGE
  $ light token-balance --mint <value> --owner <value>

FLAGS
  --mint=<value>   (required) Mint address of the token account.
  --owner=<value>  (required) Address of the token owner.
```

Displays light token account (hot), compressed light token (cold), and total balances.

### Support

- Always feel free to join the [Developer Discord](https://discord.gg/D2cEphnvcY) for help!
- For more info about the canonical indexer implementation built and maintained by Helius Labs, refer to the [Photon codebase](https://github.com/helius-labs/photon).
- For more info about Light, refer to the [documentation](https://docs.lightprotocol.com/).
