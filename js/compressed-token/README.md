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

this starts a solana-test-validator + auto-initialized the env, programs, and accounts Light needs.
It should print: "Executing command solana-test-validator...
Waiting for fees to stabilize 1..."
Keep the validator running.

Now open another terminal, activate the devenv again + move to js/compressed-token.

now run:

`pnpm emit-event:create_mint`
`pnpm emit-event:mint_to`

This runs ./tests/e2e/emit-create_mint.test.ts and ./tests/e2e/emit-mint_to.test.ts which execute a mint and compressed mint_to against the test-validator. You'll be able to index the emitted events (output utxos) according to the event rust struct. Create mint doesn't create an output utxo, mint_to doesn't use input_utxos.

`pnpm emit-event:transfer-spl` is WIP. Will merge this onto an updated branch soonTM.

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
