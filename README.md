# Light Protocol Program V2

## Tests

*Requirements:*
- solana cli v1.9.16 or higher (above 1.10.25 doesn't work right now because additional compute budget needs to be requested with an extra instruction)
  - ``sh -c "$(curl -sSfL https://release.solana.com/v1.9.16/install)"``
- anchor cli
  https://project-serum.github.io/anchor/getting-started/installation.html
  - ``npm i -g @project-serum/anchor-cli``


*Unit Tests:*
- ``cd anchor_programs/``
- ``cargo test``

*Anchor tests:*
(runs merkle tree tests located in tests/merkle_tree_program.ts)
- ``npm install``
- ``anchor test``

For repeated tests ``anchor test --skip-build`` is useful.

Check logs in anchor_programs/.anchor/program-logs

### Current State

The current user flow is separated in two actions:
- first verifying a proof plus executing protocol logic (moving funds, storing commitments onchain among others)
- second inserting those commitments as batches into the Merkle tree

The second flow is finished and ready for audit. It consists out of the
following functions in the merkle_tree_program directory:
- initialize_merkle_tree_update_state
- update_merkle_tree
- insert_root_merkle_tree
- close_merkle_tree_update_state (in case something goes wrong the update state account can be closed by the relayer)

The anchor tests currently run only the merkle_tree_program tests.

In general, both programs are finished and tested except for the spl token deposits and withdrawals.
I am about to finish those up in the coming days which might impact the verifier program.

The Merkle tree program is not affected this except the two functions initialize_new_merkle_tree_spl and withdraw_spl.


### Batched Merkle tree updates

**Problem:**

We need to writelock our Merkle tree during updating. Due to Solana network conditions this writelock can last for several minutes. In case the update fails the tree remains locked for an even longer time. This can quickly lead to a lot of backlog and failing transactions.

**Solution:**

Users don’t update the Merkle tree themselves but only validate their proof and store leaves on chain. Anytime a crank can be executed to insert several leaves at once into the Merkle tree. This way spikes in usage can be absorbed since funds can always be spent, just funds in change utxos remain frozen until the next time the update is executed.

**Current flow:**

- **User:** send data → verify ZKP → update Merkle tree → transfer funds, emit nullifiers, insert Merkle tree leaves and root

**New flow:**

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

### The architecture changes are the following:
- one Merkle tree program which can only be invoked by registered verifier programs (currently hardcoded)
- this enables us to use multiple verifier programs which invoke the Merkle tree i.e. to easily use different verifying keys
- re-write of Groth16 proof verification and poseidon hashes: they've been adapted to the
  increased Solana compute budget of 1.4 compute units per instruction. This makes the program about 30% more efficient.
- use of the anchor framework -> typescript tests -> snarkjs proofgen in tests. Easier integration testing with .ts


### Verifier Program:
- verifies Groth16 ZKPs
- invokes the Merkle Tree program to:
  - withdraw funds from a liquidity pool
  - update the Merkle tree
  - insert leaves into the Merkle tree
  - insert nullifiers
- currently deposits are handled in the verifier program

Accounts:
  VerifierState:
  - saves data between computation steps of the Groth16 verification
  - saves data for protocol logic

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
      - two encrypted utxos (222 bytes)

  AuthorityPda:
    - can register new verifier programs

  VerifierRegistryPda:
    - registers one verifier based on its derivation path



## Current State

The following implementations are ready for review. The protocol logic and access control is not.


### Poseidon Hash

The implementation is the same for only more permutations are executed within one transactions.

### Input Preparation

Our circuit has seven public inputs.
The prepared inputs implementation remains largely the same.
The biggest changes is the round constant which is increased from 4 rounds
by a multiple of 12 to 48 within one instruction. A multiple constant is specified in
anchor_programs/programs/verifier_program/src/groht16_verifier/prepare_inputs/processor.rs/L6.

The total number of rounds is 256. Since it does not divide exactly by 48 an additional instruction is necessary
which executes the remaining 16 rounds. This instruction is merged with two smaller instructions
which perform the gic additions.


### Miller Loop and Final Exponentiation

Miller loop and final exponentiation are rewritten to be better adjusted to
Solana's increased compute budget.
In both cases the original implementation is split up into steps which can be
executed within 1.4m compute units. Every step has compute costs assigned
to it which were collected through manual measurement.
Every steps increments a global total-compute-used variable which is checked
before every compute step. Before a step it checks whether enough compute is
left in the transaction. if not enough compute is left the computation is stopped and the current state is saved.

### Miller Loop
Helper variables:
// max compute budget to be used during miller loop execution.
- ml_max_compute

// keep state of the different indices in the loops of the miller loop
- outer_first_loop
- outer_second_loop
- outer_third_loop
- first_inner_loop_index
- second_inner_loop_index
- square_in_place_executed

// keep state in coeff generation from proof b which are generated on demand
// the verifying key is hardcoded therefore obtaining these coeffs is inexpensive
- outer_first_loop_coeff
- outer_second_coeff
- inner_first_coeff



### Example Final Exponentiation:

Helper variables:
- fe_instruction_index // keeps track of the executed transaction
- fe_max_compute      // defines max compute to be used within one transaction
- current_compute // collects an estimate of how many compute units have already been used
- initialized     // cyclotomic_exp is initialized
- outer_loop      // index of cyclotomic_exp loop
- cyclotomic_square_in_place // has been executed in this loop iteration


`Check if instruction was already executed`
if state.fe_instruction_index == 0 {
    `Increment current_compute variable,
    check whether enough compute is left to execute the step,
     if not stop the computation and safe the current state.`
    state.current_compute += 288464;
    if !state.check_compute_units() {
        return Ok(Some(self.f));
    }
    `Unpack variables necessary in this compute step.`
    FinalExponentiationComputeState::unpack(
        &mut state.current_compute,
        &mut self.f,
        state.f_bytes,
    );

    self.f = self.f.inverse().unwrap(); //.map(|mut f2| {

    `Mark the compute step as executed.`
    state.fe_instruction_index += 1;

}
