# Documentation V3

This repository contains 3 solana programs, two rust crates (light_verifier_sdk, Groth16-solana), a Multi-asset pool circuit and a typescript client sdk:

- merkle_tree_program (merkle tree, update logic, system config)
- verifier zero (2 inputs 2 outputs)
- verifier one (10 inputs 2 outputs)
- Groth16-solana (rust crate Groth16 verification and input prep with syscalls)
- light-verifier-sdk (rust crate light protocol logic, proof verification)
- Light_Circuits (multi asset pool written in circom, audited and partly formally verified by Veridise)

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
- close_merkle_tree_update_state (in case something goes wrong the update state account can be closed by the rpc)

# Architecture:

- one Merkle tree program which can only be invoked by registered verifier programs
- this enables us to use multiple verifier programs which invoke the Merkle tree i.e. to easily use different verifying keys
- new verifiers can be registered by the Merkle tree authority
- re-write of Groth16 proof verification with solana runtime syscalls into one transaction.

## Verifier Program Zero:

- verifies Groth16 ZKPs
- invokes the Merkle Tree program to:
  - unshield funds from a liquidity pool
  - create Merkle tree leaves account
  - insert nullifiers
- shields are handled in the verifier program
- we use lookup tables to get everything into one transaction, this does not require any changes of the program therefore it should not be security relevant as long as the correct lookup table is used by the client.

## Verifier Program One:

- verifies Groth16 ZKPs
- invokes the Merkle Tree program to:
  - unshield funds from a liquidity pool
  - create Merkle tree leaves account
  - insert nullifiers
- shields are handled in the verifier program
- creates one state account to save state between two transactions:
  - first transaction, send data except proof
  - second transction: send proof data, verify, etc.

## Merkle Tree Program:

- trusts the verifier programs -> does only access control checks
- owns merkle tree update state accounts
- owns accounts of the liquidity pools
- implements transfer logic to shield and unshield tokens
- owns Merkle tree permanent storage accounts which store the state for sparse
  Merkle trees
- registers verifier program
- only registerd verifier programs can interact with it
- owns and inserts nullifier program derived addresses
- owns and inserts merkle tree leaves pdas

### Merkle tree lock:

- init update state takes the lock if lock in merkle tree is zero or expired
- update merkle tree just resets the time after every instruction and checks that merkle tree still stores its own public key as lock, the lock could be expired but if nobody else has taken it it is ok to proceed with computing
- the same applies to the last transaction the root insertion the instruction just checks that the account is still locked not whether it is expired.

### Accounts:

**MerkleTreeUpdateState:**

- saves compute of poseidon hashes during updating the merkle tree
- is initialized via cross program invocation by a verifier program

**NullifierPda:** - nullifiers are inserted once and cannot be deleted - if a transaction tries to insert a nullifier again it will fail - this results in constant lookup time for nullifiers

**LeavesPda:** - every leaves pda stores: - two leaves (2x 32 bytes) - merkle tree publikey (32 bytes) - two encrypted utxos (256 bytes)

**AuthorityPda:** - can register new verifier programs - change permissions regarding liquidity pool registration - register liquidity pools

**VerifierRegistryPda:** - registers one verifier based on its derivation path

### Configuration Account and Instructions

The merkle tree features a config account which has permission to change certain properties.
It also stores the current index for merkle trees and registered assets such that instead of the number only the index has to be stored encrypted onchain.

**Accounts:**

1. merkle_tree_authority_pda
   This account stores the configuration of the merkle tree:

- pubkey // is the current merkle tree authority
- merkle_tree_index // is the counter how many merkle tree exist
- registered_asset_index //
- enable_permissionless_spl_tokens
- enable_permissionless_merkle_tree_registration

The merkle_tree_authority_pda has to be initialized once by the hardcoded initial merkle tree authority, by invoking the instruction: initialize merkle tree authority.

The following instructions are used to configure the merkle tree and can only be accessed by the configured authority:

- update to a new authority
- register new verifier
- register new pool types
- register new spl asset pools
- register new sol asset pools
- enable nfts
- enable permissionless spl asset pool creation
- initialize new Merkle trees
- update Lock duration

**Verifier Invoked instructions**

For these instructions the merkle tree acts like a libarary for registered verifiers. These instructions can only be invoked by registered verifiers and conduct no additional checks since these verifiers are trusted. Verifiers need to invoke the Merkle tree because it owns the pdas storing the funds as well as the nullifiers and leaves. Pdas can only be changed by the owning program.

Verified Merkle Tree instructions:

- insert_nullifiers
  inserts a number of nullifiers passed in as remaining accounts
- insert_two_leaves
  creates a leaves account which stores a pair of leaves and a message of 256 bytes which can contain 2 encrypted utxos
- unshield_spl
- unshield_sol

**Security Claims**

## Batched Merkle tree updates

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
