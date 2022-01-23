# Light Protocol

## Notes:

- All code we modified is in this repository.
- The implementation is based on the arkworks libraries mainly [ark_bn254](https://docs.rs/ark-bn254/0.3.0/ark_bn254/), [ark_ec](https://docs.rs/ark-ec/0.3.0/ark_ec/) and [ark_ff](https://docs.rs/ark-ff/0.3.0/ark_ff/).
- The majority of the code was hacked together for the hackathon thus naming is not yet consistent etc.
- This repo will not execute with the client since we are currently switching the curve from bl12-381 to bn254 (which affects most files).
- Parts of the codebase are redundant and yet to be refactored (i.e. parser & processor files).
- Unit tests work.
- Client side is not updated to snarkjs yet.
- Relayer and fees are yet to be implemented.
- Security claims represent a current state and not sufficient for mainnet launch.
- Accounts init is probably still insecure.
- The circuit is currently not in the repo. Instead refer to: [tornado_pool](https://github.com/tornadocash/tornado-pool/tree/onchain-tree/circuits).

## Security Claims (preliminary):

- Instructions can only be executed in the right order
- No double spend (nullifier hash)
- Ownership checks of accounts
- No state hijacking of ongoing computation
- Injection of external inputs only in specified instructions
- implementation of onchain groth16 verification is secure
- implementation of onchain poseidon hash is secure
- implementation of the circuit is secure

## General Description

Light cash is the first implementation using light protocol to provide privacy. A SDK for light protocol will follow soon to enable privacy not only for one application but the entire Solana ecosystem.

The vision for light protocol is embedded privacy in any blockchain application. For instance imagine wallets and dexes with a button to send a private transaction. We want to equip developers with the tools to achieve privacy.

Privacy is achieved with zero-knowledge proofs which verify that the owner of recipient address B has deposited tokens to our pool from another address A before.

A relayer will trigger the withdraw transaction, thus breaking the link between a deposit and withdrawal.

The zero-knowledge proof includes meta data such as the recipient address. In case this data is tampered with the zero-knowledge proof becomes invalid and the withdrawal fails. Therefore, Light cash is completely trustless.

The front end client which includes the user interface and proof generation with webassembly light cash features a merkle tree and zero-knowledge proof verification onchain. The zero-knowledge proof is a Groth16 proofing system and poseidon sponge hash function for the merkle tree both forked from arkworks library. Our demo works with a merkle tree of height 11 supporting 2048 deposits. The current tree height is chosen for limitations in the off chain merkle proof generation. With a more efficient algorithm off chain the system can easily support merkle tree of greater height.

## Repo Description

The light-protocol-onchain repository includes two Solana programs; a main program (program folder) and a helper program (program_prep_inputs folder) and a Client (will change completely to use snarkjs). The helper program prepares the inputs for the zero-knowledge proof verification while the main program accommodates the merkle tree leaf insert logic and zero-knowledge proof verification algorithm (miller loop and final exponentiation).

The general structure of both program repositories starts from the program entrypoint in lib.rs. From lib.rs a preprocessor function is called which you can find in either a preprocesser file for the miller loop or inside the processor file for the merkletree insert and final exponentiation. These preprocessor functions call instructions in their respective processor files. The processor function calls the actual instructions which are in merkle_tree_instructions.rs and instructions.rs for the miller loop as well as in different files for the final exponentiation (inverse.rs, utils.rs, mul_assign.rs). At the moment, several parser files exist for legacy reasons to parse data types from bytes only when necessary to save compute units.

State is stored in temp accounts during the computation and one persistent account which stores the merkle tree state. The structs for the accounts are implemented in state_\*.rs files. To save computation we have created several structs for the same account which are specialized for different use cases, such as the general computation storage (state_miller_loop.rs, state_final_exp.rs, state_merkle_tree.rs), initialization with bytes from prior compuation (state_prep_inputs.rs, state_merkle_tree.rs, state_miller_loop_transfer.rs), root and nullifier checks (state_check_nullifiers, state_merkle_tree_roots).

The verifying key is hardcoded in the program in file hard_coded_verifying_key_\*.rs for the miller loop. The part of the verifying key used to verify the correctness of the final exponentiation is hardcoded in utils.rs.


The user flow for the three actions setup, deposit and withdraw works as follows:

### Setup (Merkle tree account initialization)

A new merkle tree account can be initialized by calling the merkle tree instruction 240. This instruction simply copies the state of an initialized  hard coded bytesThe initialization bytes (init_bytes11.rs) for a new merkle tree account.

lib.rs -> merkle_tree_processor.rs -> init_bytes11.rs

Accounts:
- persisten merkle tree (InitMerkleTreeBytes struct in state_merkle_tree.rs)



### Deposit (Merkle tree leaf insertion)

At deposit the user transfers the funds and submits a hash to a merkle tree on chain. The inputs of the hash will later be used to proof a prior submission to the merkle tree. To compute the hashes to update the merkle tree we use 311 instructions within four transactions.

During the deposit an account temporarily stores the state for the hash function and another account serves as intermediary to easily check the deposited amount. The deposit amount is transferred by the user to the intermediary account which is owned by the program. The program in turn transfers the denominated amount to the merkle tree state account. To avoid write conflicts during the merkle tree update the tree account is locked for a number of blocks once a deposit starts. To avoid invalid state in case the merkle tree isn't completely updated before the lock expires the counter of leaves is incremented when the root is updated in the last instruction.

lib.rs -> merkle_tree_processor.rs -> instructions_merkle_tree.rs

Accounts:
- persistent merkle tree (MerkleTree struct in state_merkle_tree.rs
- tmp hash storage account (HashBytes struct in  state_merkle_tree.rs)
- escrow account for deposit amount


### Withdrawal (ZKP verification)

For withdrawal the user calculates a zero-knowledge proof off-chain. The proof and its public inputs which include the recipient address are sent to the onchain program which prepares the inputs and verifies the zero-knowledge proof. At successful verification funds are withdrawn to the specified recipient address.
In a shielded pool internal shielded transactions will also be possible but this is not implemented yet.

Public inputs for the ZKP verification are prepared in the Prepare_Inputs program. This program needs an account for temporary storage of the computation and the persistent merkle_tree account to check that the merkle root exists. The actual verification is performed by the main program which also executes the actual withdrawal.

program_prep_inputs

  Steps:

    - Check merkle root exists. A struct tailored to making the roots available is used to parse the merkle tree account.
    lib.rs -> pre_processor_prep_inputs.rs -> state_merkle_tree_roots.rs

    - Prepare inputs
    lib.rs -> pre_processor_prep_inputs.rs -> processor_prepare_inputs.rs -> instructions_prepare_inputs.rs


program

  Steps:

      - transfer state from prepared input account to miller loop account

      - Calculate miller loop

      lib.rs -> pre_processor_miller_loop.rs -> processor.rs -> instructions.rs -> hard_coded_verifying_key_\*.rs

      - Caculate final exponentiation

      lib.rs -> processor_part2.rs -> instructions (inverse.rs, mul_assign.rs, utils.rs)

      - check nullifier does not exist yet

      lib.rs -> processor_part2.rs -> state_check_nullifier.rs

      - check proof verification was sucessful (check result with hardcoded verifyingkey)

      - send withdrawal amount

  Accounts:

  - Merkle tree account (to check root exists, nullifier does not exists, insert nullifier)

  - Prepare inputs account for temporary storage

  - Miller loop and final exponentiation use the same account for temporary storage

## Security Checks

### **Deposit:**

- [x]  tree is initialized (assert in unpack, doesn t crash but doesn t do anything if not inited)
- [x]  user deposited the right amount into the tree account

    user transfers deposit to tmp account

    program transfers amount from tmp account to merkletree account which only works if the amount is right

    check owner of merkletree account

- [x]  check signer probably not necessary if both leaf hash submission and transfer occur in the same tx
- [x]  tree account address is hardcoded in program
- [ ]  init hash_bytes_correctly

### **Prepare Inputs:**

- [x]  reading from right merkletree account (state_merkle_tree_roots.rs line 81)
- [x]  check signer (pre_processor_prep_inputs.rs line 185)
- [x]  tx order onchain
- [x]  tmp account does not need to be checked since it is always written in
- [x]  crash when preparing inputs finished (pre_processor_prep_inputs.rs line 35)
- [ ]  check denomination vs proof value

### **Withdraw:**

**reading init data** from right prepare inputs (pre_processor_miller_loop.rs)

- [x]  checking signer is the same (line 52)
- [x]  checking owner of the account against hardcoded value ( line 47)
- [x]  checking current instruction index is 1086 aka prepared inputs finished (line 54)
- [x]  checking root has been found (line 53)

**General** (pre_processor_miller_loop.rs, processor_part_2.rs)

- [x]  check signer in every tx (Miller loop line 202, Final Exp line 58)
- [x]  tx order onchain, in state_miller_loop.rs, state_final_exp.rs
- [x]  handle crash when end of tx order is reached (Miller Loop line 29)

**Start Final Exp**

- [ ]  miller loop has finished

**check nullifier** (checks are in state_check_nullifiers.rs)

- [x]  check merkletree account is owned by program (in every check nullifier fn)
- [x]  check merkletree account is initialized (in every unpack)

a**ctual withdraw** (processor_part_2.rs)

- [x]  check merkletree account is owned by program with hardcoded value (line 85)
- [x]  check account to withdraw to is the same as used for prepared inputs (line 88)
- [x]  check nullifier has not been found or abort (line 82)


## Setup

The current tree height is set to 11. For this tree height init data and an instruction order
to insert a leaf into the tree are hardcoded. To generate a new instruction order and init data
run:
``` cd program && cargo test merkle_tree_print_init_data_and_instruction_order && cd ..```


## program



- start a local validator

```solana-test-validator --reset```


- build & deploy program inside /program

```cd program && sh deploy_program.sh```

```cd program_prep_inputs && sh deploy_program.sh```



## CLI
Does not work with current merkletree

- set up the .env file with a private key (64-byte, i.e. PRIVATE_KEY="1,1,1,1,....")
-airdrop yourself tokens
```solana airdrop 100 <publicKey>```

- inside /webassembly build wasm binary and bindings for main.js to consume

```cd Client-Js/webassembly && sh compile_wasm.sh```
```cd Client-Js```
```npm install```

- execute CLI, commands:

run this once:
```npm run-script run init_merkle_tree ```

```npm run-script run deposit SOL 1```

```npm run-script run withdraw <note> <address_to>```

#
