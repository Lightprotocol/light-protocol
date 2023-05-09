# Light Protocol Program V3

## Setup

_Requirements:_

- solana cli v1.11.10 or higher
  - `sh -c "$(curl -sSfL https://release.solana.com/v1.14.13/install)"`
- anchor cli
  https://project-serum.github.io/anchor/getting-started/installation.html
  - `yarn i -g @coral-xyz/anchor-cli`
- node v19

- Execute the build script to download a custom solana test validator and build the project:
- `sh build.sh`

## Setup Dev

- In order to properly execute the prettier format pre-commit hook, you may first need to configure light-sdk-ts/husky/pre-commit as executable:
- `chmod ug+x ./light-sdk-ts/husky/* `

## Tests

_Global:_

- `sh test.sh`

_Rust tests:_

- `cd groth16-solana/`
- `cargo test`
- `cd light-verifier-sdk/`
- `cargo test`

_Sdk tests:_

- `cd light-sdk-ts/`
- `yarn test`

_Circuit tests:_

- `cd light-circuits`
- `yarn test`

_Anchor tests:_

Tests are located in tests/ .
The default test is a functional test, setting up a test environment with a merkle tree and an spl token, conducting two deposits and withdrawals.

Tests can be executed in bulk or one by one.

**Without external validator:**

- `cd light-system-programs/`
- `yarn test`
- `yarn test-verifiers`
- `yarn test-merkle-tree`

**Manual:**

- Assuming that your clones of `solana` and `light-protocol-onchain` git
  repositories share the same parent directory and you are currently in the
  `light-protocol-onchain` directory, launch a validator with the following
  command:

1. cargo build in light-verifier-sdk (optional)
2. anchor build in light-system-programs
3. anchor build in mock-app-verifier

```
solana-test-validator \
    --reset \
    --limit-ledger-size 500000000 \
    --bpf-program J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i ./light-system-programs/target/deploy/verifier_program_zero.so \
    --bpf-program JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6 ./light-system-programs/target/deploy/merkle_tree_program.so \
    --bpf-program 3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL ./light-system-programs/target/deploy/verifier_program_one.so \
    --bpf-program GFDwN8PXuKZG2d2JLxRhbggXYe9eQHoGYoYK5K3G5tV8  ./light-system-programs/target/deploy/verifier_program_two.so  \
    --bpf-program noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV ../solana/web3.js/test/fixtures/noop-program/solana_sbf_rust_noop.so \
    --bpf-program Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS ./mock-app-verifier/target/deploy/mock_verifier.so
```

- `anchor test --skip-build --skip-deploy --skip-local-validator`

Check logs in anchor_programs/.anchor/program-logs
w

## Todos

### Swen

[ ] - add encryption to file storage (cli)

[ ] - "privamount" -> "shieldedAmount", 2 amounts passable w SPL support

[ ] - userTokenPda -> as variable input w SPL support

[ ] - relayer should recevie ixs instead of txs

[ ] - remove testaccountsetup fns (part of spl support)

[ ] - remove 2 airdrop checks for relayer recipient (currently randomly generated inside transfer/unshield), part of relayer activation
