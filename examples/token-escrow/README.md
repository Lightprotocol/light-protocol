# Token Escrow program example

An example program escrowing compressed tokens into (1) a regular solana program account, (2) into a compressed-pda account.


### Run the tests

In the monorepo root, run the build.sh script

```bash
. ./scripts/devenv.sh
./scripts/build.sh
```

Then navigate to the token-escrow directory and run the rust tests:

```bash
cd examples/token-escrow/programs/token-escrow
cargo test-sbf -- --test-threads=1
```

##  This program is unsafe, don't use it in production.
