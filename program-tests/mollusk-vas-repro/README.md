# Mollusk VAS repro

Reproduces the `virtual_address_space_adjustments` state-byte divergence for:

- program: `compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq`
- feature gate: `7VgiehxNxu53KdxgLspGQY8myE6f7UokaWa4jsGcaSz`
- example transaction: `2iT1exGBk773wicyqhoEiZZVjSBQhgxmoo2YHr5H177PCgU4TThbRrWqQg7y2g64Svxxa4v7GjDwFAQdTUFwfKCv`

The harness runs the extracted inner account-compression instructions twice:

1. with `virtual_address_space_adjustments = false`
2. with `virtual_address_space_adjustments = true`

It then compares the resulting state-tree account bytes.

The crate is intentionally a nested workspace so its Mollusk/Solana dependency
stack does not alter the root workspace dependency graph.

## Fixtures

By default the harness expects:

```text
fixtures/account-compression-mainnet.so
fixtures/35hkDgaAKwMCaxRz2ocSZ6NaUrtKkyNqU6c4RV3tYJRh.bin
fixtures/smt8TYxNy8SuhAdKJ8CeLtDkr2w6dgDmdz5ruiDw9Y9.bin
```

These are intentionally gitignored.

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

Or override paths with:

```bash
MOLLUSK_VAS_ELF=/path/to/account-compression.so \
MOLLUSK_VAS_REGISTERED_PROGRAM_ACCOUNT=/path/to/registered-program.json \
MOLLUSK_VAS_STATE_TREE_ACCOUNT=/path/to/state-tree.json \
cargo run --locked --manifest-path program-tests/mollusk-vas-repro/Cargo.toml --offline
```

## Run

```bash
RUST_LOG=off cargo run --locked --manifest-path program-tests/mollusk-vas-repro/Cargo.toml --offline
```

The harness writes:

```text
program-tests/mollusk-vas-repro/output/disabled-state-tree.raw
program-tests/mollusk-vas-repro/output/enabled-state-tree.raw
```

