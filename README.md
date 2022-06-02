#Light Protocol Program V2

## Tests

Requirements:
- solana cli v1.9.16 or higher
  sh -c "$(curl -sSfL https://release.solana.com/v1.9.16/install)"
- anchor cli
  https://project-serum.github.io/anchor/getting-started/installation.html
  npm i -g @project-serum/anchor-cli



*Unit Tests:*
- ``cd anchor_programs/``
- ``cargo test``

Anchor tests:
``npm install``
``anchor test``

For repeated tests ``anchor test --skip-build`` is useful.
Check logs in anchor_programs/.anchor/program-logs

###Current State
Both programs are not secure and are missing basic account and other security checks.

Only Groth16 ZKP verification and poseidon hashes are fully implemented right now.


###The architecture changes are the following:
- one Merkle tree program which can only be invoked by registered verifier programs (currently hardcoded)
- this enables us to use multiple verifier programs which invoke the Merkle tree i.e. to easily use different verifying keys
- re-write of Groth16 proof verification and poseidon hashes: they've been adapted to the
  increased Solana compute budget of 1.4 compute units per instruction. This makes the program about 30% more efficient.
- use of the anchor framework -> typescript tests -> snarkjs proofgen in tests. Easier integration testing with .ts


###Verifier Program:
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

###Merkle Tree Program:
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



##Current State

The following implementations are ready for review. The protocol logic and access control is not.


###Poseidon Hash

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

###Miller Loop
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



###Example Final Exponentiation:

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
