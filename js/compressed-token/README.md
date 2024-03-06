js client to interact with the compressed-token program

WIP.

### Requirements

reproduced on mac m2:

- node: v20.9.0
- rustup 1.26.0, rustc 1.73.0, cargo 1.73.0
- solana-cli 1.17.5

## Producing events for indexing

First, activate the devenv

`. ./scripts/devenv.sh`

If you're new, run

`./scripts/install.sh`

Run the monorepo build script
`./scripts/build.sh`

Go to stateless.js
`cd js/stateless.js`

and run
`pnpm run test-validator`

this starts a solana-test-validator + auto-initialized the env, programs, and accounts Light needs. Keep the validator running.

now open another terminal, enter the devenv + compressed-token,

`cd js/compressed-token`

now run:

`pnpm emit-event:mint`
`pnpm emit-event:mint_to`
`pnpm emit-event:transfer-spl`

This runs ./tests/e2e/transfer-emit-events.test.ts which executes a simple compressed sol transfer against the test-validator. You'll be able to index the emitted events (output utxos) according to the event rust structs.

### Troubleshooting

If you're having trouble building the project or cli,

- Nuke git clean -xfd (careful)

- re-run install.sh etc

- you may want to manually build the programs (anchor build)
  or manually build the cli (`pnpm run build` in ./cli) before running
  `./cli/test_bin/run test-validator`

### Other side notes

1. This is unsafe. We don't verify ZKPs yet, nor do we validate tree roots.
2. this is also what allows us to make up input-utxos for emit-event:transfer
3. on-chain runs a sumcheck on the state transition (outputs, inputs).
