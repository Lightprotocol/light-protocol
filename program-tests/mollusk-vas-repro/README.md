# Mollusk VAS repro

Reproduces the `virtual_address_space_adjustments` state-byte divergence for:

- program: `compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq`
- feature gate: `7VgiehxNxu53KdxgLspGQY8myE6f7UokaWa4jsGcaSz`
- example transaction: `2iT1exGBk773wicyqhoEiZZVjSBQhgxmoo2YHr5H177PCgU4TThbRrWqQg7y2g64Svxxa4v7GjDwFAQdTUFwfKCv`

The harness runs each case twice:

1. with `virtual_address_space_adjustments = false`
2. with `virtual_address_space_adjustments = true`

It then compares the resulting account bytes against the case's expectation.

| Case | Program | What it runs | Expected divergence |
|------|---------|--------------|---------------------|
| `mainnet-smt` | mainnet ELF | the extracted inner instructions of the mainnet transaction against the mainnet state tree | state tree diverges |
| `local-amt1-update` | local ELF | init v1 address tree + queue, insert an address, update the tree | none |
| `local-amt2-insert` | local ELF | init batched address tree, insert an address | none |
| `local-bmt1-batch-append` | local ELF | init batched state tree + output queue, insert leaves, batch append with a proof | none |

The crate is intentionally a nested workspace so its Mollusk/Solana dependency
stack does not alter the root workspace dependency graph.

## Requirements

- `target/deploy/account_compression.so` built from this checkout
  (`cargo build-sbf -p account-compression` or `./scripts/build.sh`).
- `third-party/solana-program-library/spl_noop.so`.
- A local prover: `local-bmt1-batch-append` calls `spawn_prover()`, which
  uses an already running local prover or starts one via the `light` CLI on
  `PATH`.
- The mainnet fixtures below.

## Fixtures

By default the harness expects:

```text
fixtures/account-compression-mainnet.so
fixtures/35hkDgaAKwMCaxRz2ocSZ6NaUrtKkyNqU6c4RV3tYJRh.bin
fixtures/smt8TYxNy8SuhAdKJ8CeLtDkr2w6dgDmdz5ruiDw9Y9.bin
```

The `.bin` files are raw account data. They are intentionally gitignored.

If you already have the files in `/private/tmp`, copy them once:

```bash
mkdir -p program-tests/mollusk-vas-repro/fixtures
cp /private/tmp/account-compression-mainnet.so \
  program-tests/mollusk-vas-repro/fixtures/account-compression-mainnet.so
cp /private/tmp/35hkDgaAKwMCaxRz2ocSZ6NaUrtKkyNqU6c4RV3tYJRh.bin \
  program-tests/mollusk-vas-repro/fixtures/35hkDgaAKwMCaxRz2ocSZ6NaUrtKkyNqU6c4RV3tYJRh.bin
cp /private/tmp/smt8TYxNy8SuhAdKJ8CeLtDkr2w6dgDmdz5ruiDw9Y9.bin \
  program-tests/mollusk-vas-repro/fixtures/smt8TYxNy8SuhAdKJ8CeLtDkr2w6dgDmdz5ruiDw9Y9.bin
```

Every path can be overridden:

| Variable | Default |
|----------|---------|
| `MOLLUSK_VAS_ELF` | `fixtures/account-compression-mainnet.so` |
| `MOLLUSK_VAS_LOCAL_ELF` | `../../target/deploy/account_compression.so` |
| `MOLLUSK_VAS_NOOP_ELF` | `../../third-party/solana-program-library/spl_noop.so` |
| `MOLLUSK_VAS_REGISTERED_PROGRAM_ACCOUNT` | `fixtures/35hkDgaAKwMCaxRz2ocSZ6NaUrtKkyNqU6c4RV3tYJRh.bin` |
| `MOLLUSK_VAS_STATE_TREE_ACCOUNT` | `fixtures/smt8TYxNy8SuhAdKJ8CeLtDkr2w6dgDmdz5ruiDw9Y9.bin` |
| `MOLLUSK_VAS_OUTPUT_DIR` | `output` |

## Run

```bash
RUST_LOG=off cargo run --locked --manifest-path program-tests/mollusk-vas-repro/Cargo.toml --offline
```

The run fails if any case's set of divergent accounts differs from its
expectation, or if a required account was not mutated. For every divergent
account the harness prints the differing byte ranges and writes both versions:

```text
program-tests/mollusk-vas-repro/output/<case>/<address>-disabled.raw
program-tests/mollusk-vas-repro/output/<case>/<address>-enabled.raw
```
