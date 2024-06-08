# Name Service program example

This example program creates a compressed-pda account with an address.

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
