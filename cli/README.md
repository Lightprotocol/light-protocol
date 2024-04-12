# Light CLI

CLI to interact with Light Protocol and use generalized ZK compression.

## Requirements

- Ensure you have Node.js (version 12 or later) and npm installed on your machine.

- You will need a valid Solana filesystem wallet set up at `~/.config/solana/id.json`.
If you don't have one yet, visit the Solana documentation for details: https://docs.solanalabs.com/cli/wallets/file-system 
The CLI will use this wallet as the default fee payer and mint authority.

## Installation


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

## Usage


**1. Navigate to the CLI directory and start the light test validator**

```bash
cd cli && light test-validator
```

This starts a Solana test-validator with the light system programs and accounts, a prover server, and an indexer as background processes against a clean ledger.


```bash
# Pass the -i flag to start without the indexer
light test-validator -i

# Pass the -p flag to start without the prover
light test-validator -p
```
> **Note:** The CLI currently expects the photon indexer to run at port: `8784` and the gnark-prover at port: `3001`



**2. Ensure you have sufficient localnet funds** 

```bash
# Airdrop 1 SOL
solana airdrop 1000000000

# Print your address
solana address

# Print your balance
solana balance

```

Now you're all set up to run some CLI commands :)

### Commands


#### Create a compressed token mint 

```bash
light create-mint
```
```
USAGE
  $ light create-mint [--mint-keypair <value>] [--mint-authority <value>]
    [--mint-decimals <value>]

FLAGS
  --mint-authority=<value>  Specify a path to the mint authority keypair file.
                            Defaults to your default local Solana wallet file
                            path
  --mint-decimals=<value>   [default: 9] Number of base 10 digits to the right
                            of the decimal place [default: 9]
  --mint-keypair=<value>    Provide a path to a mint keypair file. Defaults to a
                            random keypair
```

#### Mint compressed tokens to a Solana wallet

```bash
light mint-to --mint "YOUR_MINT_ADDRESS" --to "YOUR_WALLET_ADDRESS" --amount 4200000000 
```
```
USAGE
  $ light mint-to --mint <value> --to <value> --amount <value>
    [--mint-authority <value>]

FLAGS
  --amount=<value>          (required) Amount to mint, in tokens.
  --mint=<value>            (required) Specify the mint address.
  --mint-authority=<value>  Specify the filepath of the mint authority keypair.
                            Defaults to your local solana wallet.
  --to=<value>              (required) Specify the recipient address.
```



#### Transfer compressed tokens from one wallet to another

```bash
light transfer --mint "YOUR_MINT_ADDRESS" --to "YOUR_WALLET_ADDRESS" --amount 4200000000 
```

```
USAGE
  $ light transfer --mint <value> --to <value> --amount <value>
    [--fee-payer <value>]

FLAGS
  --amount=<value>     (required) Amount to send, in tokens
  --fee-payer=<value>  Specify the fee-payer account. Defaults to the client
                       keypair.
  --mint=<value>       (required) Mint to transfer
  --to=<value>         (required) Recipient address

```


#### Compress native SOL

> **Note:** Ensure the the SOL omnibus account of the Light system program is already initialized by running: `light init-sol-pool`


```bash
light compress-sol --amount 1000 --to "YOUR_WALLET_ADDRESS_BASE58"
```
```
USAGE
  $ light compress-sol --to <value> --amount <value>

FLAGS
  --amount=<value>  (required) Amount to compress in lamports.
  --to=<value>      (required) Specify the recipient address.
```

#### Decompress into native SOL

```bash
light decompress-sol --amount 42 --to "YOUR_WALLET_ADDRESS_BASE58"
```
```
USAGE
  $ light decompress-sol --to <value> --amount <value>

FLAGS
  --amount=<value>  (required) Amount to decompress in lamports.
  --to=<value>      (required) Specify the recipient address.
```
