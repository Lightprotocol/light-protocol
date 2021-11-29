# onchain-experiments
# Setup

The current tree height is set to 11. For this tree height init data and an instruction order
to insert a leaf into the tree are hardcoded. To generate a new instruction order and init data
run:
``` cd program && cargo test merkle_tree_print_init_data_and_instruction_order && cd ..```

## Notes:

- The majority of the code was hacked together for the hackathon thus naming is not consistent etc.
- this repo will not execute with the client since we are currently switching the curve from bl12-381 to bn254
- unit tests work
- Client side is not updated to snarkjs yet
- Relayer and fees are yet to be implemented
- Security claims represent a current state and not sufficient for mainnet launch
- accounts init is probably still insecure
- the circuit is currently not in the repo see https://github.com/tornadocash/tornado-pool/tree/onchain-tree/circuits

## Security Claims (preliminary):

- Instructions can only be executed in the right order
- No double spend (nullifier hash)
- Ownership checks of accounts
- No state hijacking of ongoing computation
- Injection of external inputs only in specified instructions
- implementation of onchain groth16 verification is secure
- implementation of onchain poseidon hash is secure
- implementation of the circuit is secure

## Description

Light cash is the first implementation using light protocol to provide privacy. A SDK for light protocol will follow soon to enable privacy not only for one application but the entire Solana ecosystem.

The vision for light protocol is embedded privacy in any blockchain application. For instance imagine wallets and dexes with a button to send a private transaction. We want to equip developers with the tools to achieve privacy. 

Privacy is achieved with zero-knowledge proofs which verify that the owner of recipient address B has deposited tokens to our pool from another address A before.

A relayer will trigger the withdraw transaction, thus breaking the link between a deposit and withdrawal.

The zero-knowledge proof includes meta data such as the recipient address. In case this data is tampered with the zero-knowledge proof becomes invalid and the withdrawal fails. Therefore, Light cash is completely trustless.

The front end client which includes the user interface and proof generation with webassembly light cash features a merkle tree and zero-knowledge proof verification onchain. The zero-knowledge proof is a Groth16 proofing system and poseidon sponge hash function for the merkle tree both forked from arkworks library. Our demo works with a merkle tree of height 11 supporting 2048 deposits. The current tree height is chosen for limitations in the off chain merkle proof generation. With a more efficient algorithm off chain the system can easily support merkle tree of greater height.

On chain Light Cash consists out of two Solana programs a main program and a helper. The helper program prepares the inputs for the zero-knowledge proof verification while the main program accommodate the merkle tree leaf insert logic and zero-knowledge proof verification algorithm.

The merkle tree state is stored in one persistent account.It is initialized and owned by the main program. Further accounts are necessary for temporary storage both during deposit and withdrawal processes. During the deposit an account temporarily stores the state for the hash function and another account serves as intermediary to easily check the deposited amount. The deposit amount is transferred by the user to the intermediary account which is owned by the program. The program in turn transfers the denominated amount to the merkle tree state account.

The user flow for the two actions deposit and withdraw works as follows:

At deposit the user transfers the funds and submits a hash to a merkle tree on chain. The inputs of the hash will later be used to proof a prior submission to the merkle tree. To compute the hashes to update the merkle tree we use 311 instructions within four transactions.

For withdrawal the user calculates a zero-knowledge proof off chain. This proof and its public inputs which include the recipient address are sent to the onchain program which prepares the inputs and verifies the zero-knowledge proof. At successful verification funds are withdrawn to the specified recipient address.

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
