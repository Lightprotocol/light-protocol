# README

## Introduction

This script is an example implementation of a shielded transfer using the Light Protocol and **Solana** blockchain. It demonstrates how to initialize a **Solana** wallet, request an airdrop of SOL tokens, set up a test relayer, perform a shield operation, transfer tokens to a recipient, and retrieve transaction information.

## Prerequisites

Before running this script, ensure that you have the following prerequisites:

1. **Node**.js and **npm** installed on your machine.
2. A **Solana** wallet file (JSON format) located at `~/.config/solana/id.json`. If the file doesn't exist, the script will generate a new wallet for testing purposes.

## Installation

1. Clone the repository or create a new project directory.
2. Open a terminal and navigate to the project directory.
3. Run the following command to install the required dependencies:

```shell
npm install
```

or

```shell
yarn install
```

If you want to start your project from scratch, you can add dependencies by running:

```shell
npm install @lightprotocol/zk.js @coral-xyz/anchor
```

or

```shell
yarn add @lightprotocol/zk.js @coral-xyz/anchor
```

## Configuration

Before running the script, you may need to modify the following configuration options:

- `SOLANA_PORT` (default: `"8899"`): The port on which the local **Solana** node is running. If your node is running on a different port, update this value accordingly.
- `process.env.ANCHOR_WALLET`: The path to the **Solana** wallet file. By default, it is set to `~/.config/solana/id.json`. If your wallet file is located elsewhere, modify this value accordingly.
- `LOOK_UP_TABLE`: The lookup table used for the test relayer. If the provided lookup table doesn't exist a new one will be created.

## Run

To run the project, execute the following command in the terminal:

```shell
npm run test-local
```

or

```shell
yarn run test-local
```

The command above will start a local test validator with both Light Protocol's programs and accounts initialized.

**NOTE:** `local-test` starts a **Solana** test-validator; if you want to handle a **Solana** test validator autonomously, you will need to initialize the Merkle tree and other accounts manually or using the `setMerkleTree script`;

## Overview

The script performs the following steps:

- Initializes the **Solana** wallet.
- Requests an airdrop of SOL tokens to the wallet.
- Sets up a test relayer.
- Initializes the Light Protocol provider using the **Solana** wallet and the test relayer.
- Initializes a Light Protocol user using the provider.
- Performs a shield operation to shield 1 SOL.
- Retrieves the user's balance.
- Generates a test recipient keypair.
- Requests an airdrop of SOL tokens to the recipient's public key.
- Initializes a Light Protocol provider for the recipient using the recipient's keypair and the test relayer.
- Initializes a Light Protocol user for the recipient using the provider.
- Executes a transfer of 0.25 SOL from the user to the recipient.
- Retrieves and logs the transaction hash of the transfer.
- Retrieves and logs the UTXO (Unspent Transaction Output) inbox for the test recipient.

You can use it as a reference to integrate shielded transfers into your own applications or explore the capabilities of the Light Protocol and **Solana**.

## Notes

Please note that this README provides an overview of the script's functionality and assumes familiarity with the **Solana** blockchain, the `@lightprotocol/zk.js` library, and the `@coral-xyz/anchor` library. For detailed usage and understanding, refer to our documentation.
