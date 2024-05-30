# Token Escrow program example

This example program escrows compressed tokens into (1) a regular Solana program account and (2) a compressed-pda account.

**Note:** Breaking changes to interfaces can occur. To ensure compatibility with the latest release, please check out the [latest release branch](https://github.com/Lightprotocol/light-protocol/tree/light-v0.3.0/examples/token-escrow).

### Run the tests

In the monorepo root, run the build.sh script

```bash
    source ./scripts/devenv.sh
    ./scripts/build.sh
    mkdir -p ./target/deploy
    cp ./third-party/solana-program-library/spl_noop.so ./target/deploy/spl_noop.so
    anchor build
```

Then navigate to the token-escrow directory and run the rust tests:

```bash
cd examples/token-escrow/programs/token-escrow
cargo test-sbf -- --test-threads=1
```

## This program is unsafe; don't use it in production.
