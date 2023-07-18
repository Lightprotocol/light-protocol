[![Light Protocol](assets/logo.svg)](https://lightprotocol.com)

# Light Protocol

[![Discord](https://img.shields.io/discord/892771619687268383?label=discord&logo=discord)](https://discord.gg/WDAAaX6je2)
[![Workflow Status](https://github.com/Lightprotocol/light-protocol-onchain/workflows/programs-test/badge.svg)](https://github.com/Lightprotocol/light-poseidon/actions?query=workflow)

**The ZK Layer for Solana**

Light is a zkLayer enabling private program execution, purpose-built for Solana. 

Developers can use Light to build applications such as 

- fast private payments
- encrypted orderbooks
- public on-chain games with private state

## Development environment

There are three ways of setting up the development environment:

* [devenv.sh script](#devenv.sh) - the most recommended one, both for Linux and
  macOS. Works with Bash and zsh.
* [Development Containers](#development-containers) - recommended on Linux,
  unfortunately has performance problems on macOS.
* [Manual setup](#manual-setup) - not recommended, but may be useful if the
  methods above don't work for you.

### devenv.sh

The easiest way to setup the development environment is to use our scripts
and development environment.

First, install the dependencies (they are going to be installed in `.local`
directory inside your repository clone):

```shell
./scripts/install.sh
```

Then, optionally, you can activate the development environment:

```shell
. ./scripts/devenv.sh
```

(The scripts mentioned later, like `./scripts/build.sh` or `./scripts/test.sh`
are going to work even without activating the development environment with
`source devenv.sh`, they take care of setting the correct `PATH` variable.)

If you activated the development environment, you can manually run commands
like `yarn`, `cargo`, `solana`, `solana-test-validator`. They are going to
use the dependencies installed in `.local` directory, so even if you have
different global installations, they are not going to interfere.

### Development Containers

Light Protocol fully embraces [Development Containers](https://containers.dev/),
providing a ready-to-use
[Docker container image](https://github.com/Lightprotocol/dockerfiles/pkgs/container/devcontainer)
that comes pre-configured with all necessary dependencies for building and testing.

Support for Development Containers (either native or through a plugin) is
provided by the following IDEs and editors:

* [Visual Studio Code](https://code.visualstudio.com/docs/devcontainers/containers)
* [Neovim](https://github.com/esensar/nvim-dev-container)
* [Emacs](https://github.com/emacs-lsp/lsp-docker)

### Manual setup

If you still want to setup dependencies manually, these are the requirements:

* [Rust installed with Rustup](https://rustup.rs/), stable and nightly toolchains
* [NodeJS](https://nodejs.org/) [(16.16 LTS)](https://nodejs.org/en/blog/release/v16.16.0)
* [Anchor](https://www.anchor-lang.com/) [(0.26.0)](https://crates.io/crates/anchor-cli/0.26.0)

## Building

To build the project, use the following commands:

```bash
./scripts/build.sh
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
./scripts/test.sh
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
Merkle tree and an spl token, conducting two shields and unshields.

Tests can be executed in bulk or one by one.

```bash
cd light-system-programs/
yarn test
yarn test-verifiers
yarn test-merkle-tree
```
