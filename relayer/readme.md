1. Start validator (see other readme)

```
../solana/validator/solana-test-validator \
    --reset \
    --limit-ledger-size 500000000 \
    --bpf-program J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i ./light-system-programs/target/deploy/verifier_program_zero.so \
    --bpf-program JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6 ./light-system-programs/target/deploy/merkle_tree_program.so \
    --bpf-program 3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL ./light-system-programs/target/deploy/verifier_program_one.so \     
    --bpf-program GFDwN8PXuKZG2d2JLxRhbggXYe9eQHoGYoYK5K3G5tV8  ./light-system-programs/target/deploy/verifier_program_two.so \
    --bpf-program noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV ../solana/web3.js/test/fixtures/noop-program/solana_sbf_rust_noop.so \
```

2. Airdrop cmd (might have to fund your own idjson keypair)

```

solana airdrop 100000 ZBUKxVWviAJBy12edp5H6kvhcatGYW3BV4ijbgxpVSq && solana airdrop 100000 ALA2cnz41Wa2v2EYUdkYHsg7VnKsbH1j7secM5aiP8k && solana airdrop 100000 8Ers2bBEWExdrh7KDFTrRbauPbFeEvsHz3UX4vxcK9xY && solana airdrop 10000 BEKmoiPHRUxUPik2WQuKqkoFLLkieyNPrTDup5h8c9S7

```

3. `Npm I` && `Npx tsc` in /relayer

4. `Node dist/index.js` to start relayer on port 3331

- This runs the setup script and starts the relayer (which can then be called by the sdk)
