# Light CLI

wip.

## Installation

To use Light CLI, you need to have Node.js (version 12 or later) and npm (Node Package Manager) installed on your machine.

Please compile the CLI from source.

`. ./scripts/devenv`

`./scripts/install.sh`

`./scripts/build.sh`

## Usage

Note: currently, you have to start the light-test-validator, gnark-prover, and photon indexer outside the CLI binary:

`cd js/stateless.js`

`pnpm run pretest:e2e`

This will reset and start the validator, prover, and indexer on a clean ledger.

Alternatively, to start only the light-test-validator and the gnark-prover, run:

`./cli/test_bin/run test-validator -p -i && pnpm gnark-prover`

Note: the CLI currently expects the photon indexer to run at port: 8784, and the gnark-prover at port: 3001

Once you've started all services, in the same or a separate terminal window, go to the cli directory:

`cd cli`

Ensure that the CLI is built.
Also ensure that you have a local solana wallet set up at ~/.config/solana/id.json. (see solana documentation for how to create one). This wallet will be used by the CLI as default feePayer and mintAuthority.

Run `solana address` using the solana-cli to print your id.json/wallet address.
To ensure you have enough localnet funds: run `solana aidrop 10000000`

You can now create test-data against the test-ledger and photon with the following commands:

`./test_bin/run create-mint`

This will create a random mint and print its mint address.
You can then mint some tokens to your wallet.

`./test_bin/run mint-to --mint "YOUR_MINT_ADDRESS_BASE58" --amount 4242 --to "YOUR_WALLET_ADDRESS_BASE58"`

Next, you can transfer some of your compressed tokens to another solana wallet:

`./test_bin/run transfer --mint "YOUR_MINT_ADDRESS_BASE58" --amount 3 --to "5bdFnXU47QjzGpzHfXnxcEi5WXyxzEAZzd1vrE39bf1W"`

Other commands include:

`./test_bin/run init-sol-pool` (must be run once before compressing lamports)

`./test_bin/run compress-sol --amount 1000 --to "YOUR_WALLET_ADDRESS_BASE58"`

`./test_bin/run decompress-sol --amount 42 --to "YOUR_WALLET_ADDRESS_BASE58"`
