js clients for the light protocol system programs

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

Go to /cli and build the CLI
`cd /cli` && `pnpm run build`

Now, go to stateless.js
`cd ../js/stateless.js`

and run
`pnpm run test-validator`

this starts a solana-test-validator + auto-initialized the env, programs, and accounts Light needs. Keep the validator running.

now open another terminal, enter the devenv + js/stateless.js again.

then run:

`pnpm emit-event:transfer`

This runs ./tests/e2e/transfer-emit-events.test.ts once which executes a simple compressed sol transfer against the test-validator. You can now index the emitted events (output utxos) according to the event rust structs. You can adapt the variables used and rounds inside the test file.

### Troubleshooting

If you're having trouble building the project or cli,

- Consider nuking your git cache `git clean -xfd` (careful) and re-run the steps from the start.

This should resolve any issues. Otherwise,

- you may also want to manually build the programs (anchor build). The IDLs will be auto-generated into stateless.js/src/idls/

- if you're running into issues with the CLI (e.g. if you're seeing "Warning: test-validator is not a light command.". before re-building the CLI or running `./cli/test_bin/run test-validator`, consider ensuring that the cli's dependencies (zk.js, account.rs) are built (`pnpm run build` in their respective directory)

### Other notes

On-chain we don't verify the ZKP yet, nor do we check tree roots. therefore we can make up input-utxos with random leafIndices/blindings and root indices for emit-event:transfer
