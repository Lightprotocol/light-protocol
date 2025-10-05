[![Light Protocol](assets/logo.svg)](https://lightprotocol.com)

# Light Protocol

[![Twitter](https://img.shields.io/twitter/follow/LightProtocol)](https://x.com/lightprotocol)
[![Discord](https://img.shields.io/discord/892771619687268383?label=discord&logo=discord)](https://discord.gg/WDAAaX6je2)
[![Workflow Status](https://github.com/Lightprotocol/light-protocol/actions/workflows/rust.yml/badge.svg)](https://github.com/Lightprotocol/light-protocol/actions?query=workflow)
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/Lightprotocol/light-protocol)

**The ZK Compression Protocol for Solana**

Developers can use Light to create rent-free tokens and PDAs on Solana without
sacrificing performance, security and composability.

Documentation is available here: www.zkcompression.com.

Our friends at Helius maintain the official ZK Compression indexer [here](https://github.com/helius-labs/photon).

## Examples

- [program-examples](https://github.com/Lightprotocol/program-examples)
- [example-nodejs-client](https://github.com/Lightprotocol/example-nodejs-client)
- [example-web-client](https://github.com/Lightprotocol/example-web-client)

## Custom Circuits

ZK Compression works with any regular onchain program on Solana. You don't have
to know how to use ZK, the protocol just uses it under the hood. However, if you
are building a custom ZK based app, you can also compose with compressed state
inside your circuit.

## Verifiable Build

Prerequisites:

- solana-verify
- docker
  Install `solana-verify` with `cargo-install solana-verify` or see [github](https://github.com/Ellipsis-Labs/solana-verifiable-build) for alternative install instructions.
  See https://docs.docker.com/engine/install/ for `docker` install instructions.

```
./scripts/build-verifiable.sh
```

## Verify Deployment

Release 1.0 commit hash: 1cb0f067b3d2d4e012e76507c077fc348eb88091

```
$ solana-verify verify-from-repo --program-id Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX -u main --library-name light_registry --commit-hash 1cb0f067b3d2d4e012e76507c077fc348eb88091 https://github.com/Lightprotocol/light-protocol
```

```
$ solana-verify verify-from-repo --program-id compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq -u main --library-name account_compression --commit-hash 1cb0f067b3d2d4e012e76507c077fc348eb88091 https://github.com/Lightprotocol/light-protocol
```

```
$ solana-verify verify-from-repo --program-id SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7 -u main --library-name light_system_program --commit-hash 1cb0f067b3d2d4e012e76507c077fc348eb88091 https://github.com/Lightprotocol/light-protocol
```

```
$ solana-verify verify-from-repo --program-id cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m -u main --library-name light_compressed_token --commit-hash 1cb0f067b3d2d4e012e76507c077fc348eb88091 https://github.com/Lightprotocol/light-protocol
```

## Security

The released Light Protocol programs have been audited, and the Light Protocol circuits are formally verified:

- OtterSec (Programs audit #1): [View Full Report](https://github.com/Lightprotocol/light-protocol/tree/main/audits/ottersec_v1_audit.pdf)
- Neodyme (Programs audit #2): [View Full Report](https://github.com/Lightprotocol/light-protocol/tree/main/audits/neodyme_v1_audit.pdf)
- Zellic (Programs audit #3): [View Full Report](https://github.com/Lightprotocol/light-protocol/blob/main/audits/zellic_v1_audit.pdf)
- Reilabs (Circuits Formal verification): [View Full Report](https://github.com/Lightprotocol/light-protocol/tree/main/audits/reilabs_circuits_formal_verification_report.pdf)

Note: All other tooling, such as light-sdk-macros and light-sdk, are in active development and unaudited.

## Development environment

There are three ways of setting up the development environment:

- [devenv.sh script](#devenv.sh) - the most recommended one, both for Linux and
  macOS. Works with Bash and zsh.
- [Development Containers](#development-containers) - recommended on Linux,
  unfortunately has performance problems on macOS.
- [Manual setup](#manual-setup) - not recommended, but may be useful if the
  methods above don't work for you.
- Windows is not supported.

### Prerequisites:

- Ubuntu, `sudo apt-get install lld clang`
- Fedora, `sudo dnf install clang lld`
- Arch, `sudo pacman -S lld clang`
- Mac, `brew install llvm`

### devenv.sh

The easiest way to setup the development environment is to use our scripts
and development environment.

First, install the dependencies (they will be installed in the `.local`
directory inside your repository clone).

```
./scripts/install.sh
```

By default, this will install a subset of gnark keys with Merkle tree heights sufficient for running tests. If you need the full set of production keys, you can use the --full-keys flag:

```
./scripts/install.sh --full-keys
```

Note: The default subset of keys is adequate for most development and testing purposes. The full set of keys is larger and includes additional Merkle tree heights used in production environments.

Then, activate the development environment:

```
./scripts/devenv.sh
```

Then follow the sections below, which describe the usage of `build.sh` and
`test.sh` scripts.

When the development environment is active, you can manually run commands
like `pnpm`, `cargo`, `solana`, `solana-test-validator`. They are going to
use the dependencies installed in `.local` directory, so even if you have
different global installations, they are not going to interfere.

### Development Containers

Light Protocol fully embraces [Development Containers](https://containers.dev/),
providing a ready-to-use
[Docker container image](https://github.com/Lightprotocol/dockerfiles/pkgs/container/devcontainer)
that comes pre-configured with all necessary dependencies for building and testing.

Support for Development Containers (either native or through a plugin) is
provided by the following IDEs and editors:

- [Visual Studio Code](https://code.visualstudio.com/docs/devcontainers/containers)
- [Neovim](https://github.com/esensar/nvim-dev-container)
- [Emacs](https://github.com/emacs-lsp/lsp-docker)

### Manual setup

If you still want to setup dependencies manually, these are the requirements:

- [Rust installed with Rustup](https://rustup.rs/), stable and nightly toolchains
- [NodeJS](https://nodejs.org/) [(20.9.0 LTS)](https://nodejs.org/en/blog/release/v20.9.0)
- [Anchor](https://www.anchor-lang.com/) [(0.29.0)](https://crates.io/crates/anchor-cli/0.29.0)

If you are using Ubuntu and encounter errors during the build process, you may need to install additional dependencies. Use the following command:

```
sudo apt install build-essential autoconf automake libtool zlib1g-dev pkg-config libssl-dev
```

## Building

To build the project, use the following commands:

```bash
./scripts/build.sh
```

## Solana keypair

Before doing any development or running any tests, you need to generate a new
local keypair:

```bash
solana-keygen new -o ~/.config/solana/id.json
```

## Tests

### Global

```bash
./scripts/test.sh
```

### Program tests

Program tests are located in program-tests.
Many tests start a local prover server.
To avoid conflicts between local prover servers run program tests with `--test-threads=1` so that tests are executed in sequence.

```bash
cargo test-sbf -p account-compression-test -- --test-threads=1
```

### SDK tests

```bash
cd js/stateless.js
pnpm test
```

```bash
cd js/compressed-token.js
pnpm test
```

For more support from the community and core developers, open a GitHub issue or join the Light Protocol
Discord: [https://discord.gg/x4nyjT8fK5](https://discord.gg/x4nyjT8fK5)
