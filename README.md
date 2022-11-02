# Light Protocol Program V3

## Notes
TODO:
- kick out ethers dep integrity hash
- write testing class in TS

PITTFALLS:
- leaves are 16 bytes bigger now
-

./validator/solana-test-validator --reset --limit-ledger-size 500000000     --bpf-program J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i         /home/ananas/test_light/light-protocol-onchain/anchor_programs/target/deploy/verifier_program_zero.so --bpf-program JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6         /home/ananas/test_light/light-protocol-onchain/anchor_programs/target/deploy/merkle_tree_program.so --bpf-program 3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL  /home/ananas/test_light/light-protocol-onchain/anchor_programs/target/deploy/verifier_program_one.so

anchor test --skip-build --skip-deploy --skip-local-validator

Lookup table:
token_program
authority
pre_inserted_leaves_index
merkle_tree
rent
system programs
program_merkle_tree


## Docs V3



## Tests

*Requirements:*
- solana cli v1.11.10 or higher
  - ``sh -c "$(curl -sSfL https://release.solana.com/v1.9.16/install)"``
- anchor cli
  https://project-serum.github.io/anchor/getting-started/installation.html
  - ``npm i -g @project-serum/anchor-cli``

github PR with syscalls


*Unit Tests:*
- ``cd anchor_programs/``
- ``cargo test``

*Anchor tests:*

Tests are located in tests/. There are four test files with one for the
merkle tree program, one for the verifier program, and one each to run
a longer test with random values for transactions with native sol and spl tokens
each (infinite_sol_test, infinite_spl_test).
By default anchor test runs the verifier_program test. The test files can be
switched manually at the bottom of Anchor.toml.

- ``npm install``

- ./validator/solana-test-validator --reset --limit-ledger-size 500000000     --bpf-program J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i         /home/ananas/test_light/light-protocol-onchain/anchor_programs/target/deploy/verifier_program_zero.so --bpf-program JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6         /home/ananas/test_light/light-protocol-onchain/anchor_programs/target/deploy/merkle_tree_program.so --bpf-program 3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL  /home/ananas/test_light/light-protocol-onchain/anchor_programs/target/deploy/verifier_program_one.so

- ``anchor test --skip-build --skip-deploy --skip-local-validator``

For repeated tests ``anchor test --skip-build`` is useful.

Check logs in anchor_programs/.anchor/program-logs


### General

This repository contains 3 solana programs, two rust crates (), a Multi-asset pool circuit and a typescript client sdk:
- merkle_tree_program (merkle tree, update logic, system config)
- verifier zero (2 inputs 2 outputs)
- verifier one (10 inputs 2 outputs)
- Groth16-solana (rust crate Groth16 verification and input prep with syscalls)
- light-verifier-sdk (rust crate light protocol logic, proof verification)
- Light_Circuits (multi asset pool written in circom, audited and partly formally verified by Veridise)
-

The user flow is separated in two actions:
- first verifying a proof plus executing protocol logic (moving funds, storing commitments onchain among others)
- second inserting those commitments as batches into the Merkle tree

The verifier program verifies a ZKP, inserts nullifiers, transfers funds, and
inserts merkle tree leaves. It works exactly the same way as in the last version
of Lightprotocol except that it occurs in one or two transactions depending on the number of input utxos 2 or 10,
leaves are only saved on chain but not inserted into the Merkle tree yet. The insertion of these leaves takes place in the second step and can be batched.
2 Inputs:
- one transaction
10 Inputs:
- createVerifierState (sends data)
- lastTx (executes protocol logic atomically)

The second flow consists out of the following functions in the merkle_tree_program directory:
- initialize_merkle_tree_update_state
- update_merkle_tree
- insert_root_merkle_tree
- close_merkle_tree_update_state (in case something goes wrong the update state account can be closed by the relayer)



## Architecture:
- one Merkle tree program which can only be invoked by registered verifier programs
- this enables us to use multiple verifier programs which invoke the Merkle tree i.e. to easily use different verifying keys
- new verifiers can be registered by the Merkle tree authority
- re-write of Groth16 proof verification with solana runtime syscalls into one transaction.


### Verifier Program:
- verifies Groth16 ZKPs
- invokes the Merkle Tree program to:
  - withdraw funds from a liquidity pool
  - update the Merkle tree
  - insert leaves into the Merkle tree
  - insert nullifiers
- deposits are handled in the verifier program


### Merkle Tree Program:
- trusts the verifier programs -> does only access control checks
- owns merkle tree update state accounts
- owns accounts of the liquidity pools
- implements transfer logic to deposit and withdraw tokens
- owns Merkle tree permanent storage accounts which store the state for sparse
  Merkle trees
- registers verifier program
- only registerd verifier programs can interact with it
- owns and inserts nullifier program derived addresses
- owns and inserts merkle tree leaves pdas

Merkle tree lock:
- init update state takes the lock if lock in merkle tree is zero or expired
- update merkle tree just resets the time after every instruction and checks that merkle tree still stores its own public key as lock, the lock could be expired but if nobody else has taken it it is ok to proceed with computing
- the same applies to the last transaction the root insertion the instruction just checks that the account is still locked not whether it is expired.


Accounts:
  MerkleTreeUpdateState:
  - saves compute of poseidon hashes during updating the merkle tree
  - is initialized via cross program invocation by a verifier program

  NullifierPda:
    - nullifiers are inserted once and cannot be deleted
    - if a transaction tries to insert a nullifier again it will fail
    - this results in constant lookup time for nullifiers

  LeavesPda:
    - every leaves pda stores:
      - two leaves (2x 32 bytes)
      - merkle tree publikey (32 bytes)
      - two encrypted utxos (256 bytes)

  AuthorityPda:
    - can register new verifier programs

  VerifierRegistryPda:
    - registers one verifier based on its derivation path




### Batched Merkle tree updates

**Problem:**

We need to writelock our Merkle tree during updating. Due to Solana network conditions this writelock can last for several minutes. In case the update fails the tree remains locked for an even longer time. This can quickly lead to a lot of backlog and failing transactions.

**Solution:**

Users don’t update the Merkle tree themselves but only validate their proof and store leaves on chain. Anytime a crank can be executed to insert several leaves at once into the Merkle tree. This way spikes in usage can be absorbed since funds can always be spent, just funds in change utxos remain frozen until the next time the update is executed.

**Flow:**

- **User:** send data → verify ZKP → transfer funds, emit nullifiers, and queue Merkle tree leaves (marked as account type 7), a new account called PreInsertedLeavesIndex saves the current queued leaves index
- **Crank:** update Merkle tree → insert new Merkle root and mark leaves as inserted (account type 4)

**Algorithm:**

leaves are marked by modifying the account type to a different number which represents uninserted leaves

clone instructions to test repo

I need a function to calculate how many transactions I need to send to conduct a batch update.

1. insert all leaves into tmp account
    1. all leaves are saved in an array
    2. upper limit per 16 leaves
    3. pass-in leaves accounts as remaining accounts, loop over those to copy
2. insert 2 leaves (calculate the first hash)
    1. tmp account leaves index +=2
3. update tree until getting a zero value on the right
    1. increase leaves_insert_index if one is inserted
    2. reset instruction index
    3. increase leaves index in tmp account
4. repeat until no more leaves to insert
5. update the rest of the tree
    1. at root insert merkle tree leaves index = tmp account leaves index
    2. fees are taken from leaves accounts
    3. at the end mark leaves as inserted

**Security Claims**

    **CreateUpdateState**
    1. leaves can only be inserted in the correct index order
    2. leaves cannot be inserted twice
    3. leaves are queued for a specific tree and can only be inserted in that tree
    4. lock is taken and cannot be taken again before expiry
    5. Merkle tree is registered

    **Update**
    6. signer is consistent
    7. is locked by update state account
    8. merkle tree is consistent

    **Last Tx**
    9. same leaves as in first tx are marked as inserted
    10. is in correct state
    11. is locked by update state account
    12. merkle tree is consistent
    13. signer is consistent
