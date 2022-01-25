# Light Protocol

## DISCLAIMER: THIS SOFTWARE IS NOT AUDITED. Do not use in production!

## Tests
- cargo test-bpf deposit_should_succeed
- cargo test-bpf withdrawal_should_succeed

## General Description

The Light Protocol program verifies zkSNARK proofs to enable anonymous transactions on Solana. 

An SDK will follow soon. Developers will be able to build solana-based programs on top of private transactions.
If you're a developer interested in using or integrating with the program, reach out to us: [Discord community](https://discord.gg/WDAAaX6je2)  /  [Twitter](https://twitter.com/LightProtocol)



Zero-knowledge proofs verify that the owner of recipient address B has deposited tokens to a shielded pool (similar to Zcash) from another address A before.
The zero-knowledge proof includes meta data such as the recipient address. In case this data is tampered with the zero-knowledge proof becomes invalid and the withdrawal fails. Therefore, Light Protocol is completely trustless.

### Notes:
- The implementation of the groth16_verifier is based on the arkworks libraries, mainly [ark_bn254](https://docs.rs/ark-bn254/0.3.0/ark_bn254/), [ark_ec](https://docs.rs/ark-ec/0.3.0/ark_ec/) and [ark_ff](https://docs.rs/ark-ff/0.3.0/ark_ff/).
- The implementation of the poseidon hash is based on [arkworks_gadgets](https://docs.rs/arkworks-gadgets/0.3.14/arkworks_gadgets/poseidon/circom/index.html)
- Light uses a circuit based on [tornado_pool](https://github.com/tornadocash/tornado-pool/tree/onchain-tree/circuits).
