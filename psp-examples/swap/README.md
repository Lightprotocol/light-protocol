# Swap 

This is a TypeScript implementation of the private swap. This example is built on the Solana blockchain and is bootstrapped using the [Light CLI](https://www.npmjs.com/package/@lightprotocol/cli) (which also leverages the Anchor framework).  

It uses [Light Protocol v3](https://github.com/Lightprotocol/light-protocol) for private state and state transitions. This allows the swaps to be executed fully on-chain.

## Prerequisites

Before running the code, ensure that you have the following installed on your machine:
– node.js, yarn
- circom
- rust
- cargo-expand (```cargo install cargo-expand```)
- solana-cli >= 1.16.4

## Setup

1. Install the required dependencies using npm or yarn:
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

## Contributing

Contributions are welcome. Please open an issue or submit a pull request on GitHub.
