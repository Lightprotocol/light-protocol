# Status

currently not maintained

# private-compressed-account

This PSP allows you to insert values into a Merkletree and then prove the inclusion of these values via ZKPs.

## Prerequisites

Before running the code, ensure that you have the following installed on your machine:

- node.js, yarn

- [circom](https://docs.circom.io/getting-started/installation/) > 2.0.0

- [Light CLI](https://www.npmjs.com/package/@lightprotocol/cli) >= 0.1.1-alpha.20

- rust

- cargo-expand (cargo install cargo-expand)

- solana-cli >= 1.16.4


## Run

1. Install the project's dependencies using yarn:

`yarn`

2. Build circuits:

`yarn build`

3. Execute the test suite using the following command:

`yarn test`

## User flow

1.  Insert a value into the Merkle tree. `(Hashed value, Poseidon([value]))`
2.  Prove inclusion of the value (A program can invoke this instruction via cpi to verify the property it wants to verify.)

The PSP has two circuits and two instructions to verify proofs.

1.  Insert & update Merkle tree

    ⁃ PublicInputs: `(oldRoot,newRoot)`

    ⁃ Private inputs: `(value, )`

2.  Prove inclusion

    ⁃ Public inputs: `(Merkle tree root)`

    ⁃ Private inputs: `(Merkle tree path, Value)`

## Notes

- Merkle tree is append-only

Current proof generation performance (not optimized, m2 mac, node env):

- (1) Merkle tree update, height 18: `.5-1s`
- (2) Inclusion proof: `.5-1s`

- [ ] add benchmarks after proof generation optimizations for different tree heights and machines.
