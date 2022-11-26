# Light Protocol Program V3

## Tests

*Requirements:*
- solana cli v1.11.10 or higher
  - ``sh -c "$(curl -sSfL https://release.solana.com/v1.11.10/install)"``
- anchor cli
  https://project-serum.github.io/anchor/getting-started/installation.html
  - ``npm i -g @project-serum/anchor-cli``
- node v14

solana version with syscalls in the same
- `` git clone https://github.com/ananas-block/solana.git``
- `` git checkout alt_bn128_precompiles``


*Unit Tests:*
- ``cd anchor_programs/``
- ``cargo test``
- ``cd groth16-solana/``
- ``cargo test``
- ``cd light-verifier-sdk/``
- ``cargo test``

*Anchor tests:*

Tests are located in tests/ and will take several minutes to complete.
Several tests are skipped to decrease the overall testing time.
By default a deposit merkle tree update and withdrawal are executed with random values for deposits and fees. It is recommended to restart the validator before rerunning tests.
Further tests are:
- Initialize Merkle Tree Test
  tests the configuration instructions and merkle tree initialization
- Deposit 10 utxos
  tests and runs a deposit with 10 input utxos
- Update Merkle Tree Test
  tests the merkle tree update process (CreateUpdateState, updateMerkleTree, insertRootMerkleTree)
- Withdraw 10 utxos
  tests and runs a withdrawal with 10 input utxos

- ``npm install``

- ./validator/solana-test-validator --reset --limit-ledger-size 500000000     --bpf-program J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i         /home/ananas/test_light/light-protocol-onchain/light-system-programs/target/deploy/verifier_program_zero.so --bpf-program JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6         /home/ananas/test_light/light-protocol-onchain/light-system-programs/target/deploy/merkle_tree_program.so --bpf-program 3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL  /home/ananas/test_light/light-protocol-onchain/light-system-programs/target/deploy/verifier_program_one.so

- ``anchor test --skip-build --skip-deploy --skip-local-validator``


Check logs in anchor_programs/.anchor/program-logs
