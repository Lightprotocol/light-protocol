[![Light Protocol](assets/logo.svg)](https://lightprotocol.com)

# Light Protocol

[![Discord](https://img.shields.io/discord/892771619687268383?label=discord&logo=discord)](https://discord.gg/WDAAaX6je2)
[![Workflow Status](https://github.com/Lightprotocol/light-protocol-onchain/workflows/programs-test/badge.svg)](https://github.com/Lightprotocol/light-poseidon/actions?query=workflow)

**The Privacy Layer for Solana**

Light Protocol powers apps with fast and secure on-chain privacy and compliance
controls so that web3 can go mainstream.

## Development environment

### Development Containers

Light Protocol supports [Development Containers](https://containers.dev/) and provides
[a container image](https://hub.docker.com/repository/docker/vadorovsky/lightprotocol-dev)
with all dependencies which are needed for building and testing.

Visual Studio Code comes with out of the box support for Development Containers,
but they are also supported by other editors:

* [Neovim](https://github.com/esensar/nvim-dev-container)

### Manual setup

If you still want to setup dependencies manually, these are the requirements:

* [Rust installed with Rustup](https://rustup.rs/), stable and nightly toolchains
* [NodeJS](https://nodejs.org/) [(16.16 LTS)](https://nodejs.org/en/blog/release/v16.16.0)
* [Anchor](https://www.anchor-lang.com/) [(0.26.0)](https://crates.io/crates/anchor-cli/0.26.0)

## Building

To build the project, use the following commands:

```bash
./build.sh
./build-sdk.sh
```

## Git hook

In order to properly execute the prettier format pre-commit hook, you may first
need to configure light-zk.js/husky/pre-commit as executable:

```bash
chmod ug+x ./light-zk.js/husky/*
```

## Solana keypair

Before doing any development or running any tets, you need to generate a new
local keypair:

```bash
solana-keygen new -o ~/.config/solana/id.json
```

## Tests

### Global

```bash
./test.sh
```

### Rust tests

```bash
cd light-verifier-sdk/
cargo test
```

### SDK tests

```bash
cd light-zk.js/
yarn test
```

### Circuit tests

```bash
cd light-circuits
yarn test
```

### Anchor tests

Tests are located in `tests/` directory.

The default test is a functional test, setting up a test environment with a
Merkle tree and an spl token, conducting two deposits and withdrawals.

Tests can be executed in bulk or one by one.

```bash
cd light-system-programs/
yarn test
yarn test-verifiers
yarn test-merkle-tree
```
