A private payment streaming program on Solana using Light protocol v3.

## Prerequisites

Before running the code, ensure that you have the following installed on your machine:
– node.js, yarn

- circom
- rust
- cargo-expand (`cargo install cargo-expand`)
- solana-cli = 1.16.4

## Overview

1. **Light Circuit**: The custom Light circuit `./circuit/pspPaymentStreaming.light` is used to define the logic of the PSP (see the .light file for detailed documentation of the code). At compilation, it expands into the necessary circuit files + on-chain program.

2. **Test Setup**: The code performs an airdrop of Solana tokens to specific program addresses required for verification purposes.

3. **Create and Spend Program UTXO Test**: Initializes a light user and generates a compressed UTXO (by shielding SPL tokens into the Light escrow).

4. **Payment Streaming Test**: Sets up a payment stream client, initializes the stream and calculates the required parameters for streaming. The client then stores the initial program UTXO and checks its commitment hash. It then collects the stream for the current slot and executes the UTXO action, effectively streaming the payment.

## How to Run the Code

1. Install the required dependencies using npm:

```bash
yarn install
```

2. Build circuits:

```
yarn build
```

3. Execute the test suite using the following command:

```bash
yarn test
```

## Common errors

- **error: package `solana-program v1.16.5` cannot be built because it requires rustc 1.68.0 or newer, while the currently active rustc version is 1.65.0-dev**

  Please install [solana-cli 1.16.4](https://docs.solana.com/cli/install-solana-cli-tools) or newer.

- **error: no such command: `expand`**

  Please install cargo-expand: `cargo install cargo-expand`.
