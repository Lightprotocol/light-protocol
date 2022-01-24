# Light Protocol

## DISCLAIMER: THIS SOFTWARE IS NOT AUDITED. Do not use in production!

## Tests
- cargo test-bpf deposit_should_succeed
- cargo test-bpf withdrawal_should_succeed

## General Description

Light cash is the first implementation using light protocol to provide privacy. A SDK for light protocol will follow soon to enable privacy for the entire Solana ecosystem.

The vision for light protocol is embedded privacy in any blockchain application. For instance imagine wallets and dexes with a button to send a private transaction. We want to equip developers with the tools to achieve privacy.

Privacy is achieved with zero-knowledge proofs which verify that the owner of recipient address B has deposited tokens to our pool from another address A before.

A relayer will trigger the withdraw transaction, thus breaking the link between a deposit and withdrawal.

The zero-knowledge proof includes meta data such as the recipient address. In case this data is tampered with the zero-knowledge proof becomes invalid and the withdrawal fails. Therefore, Light cash is completely trustless.

### Notes:
- The implementation of the groth16_verifier is based on the arkworks libraries mainly [ark_bn254](https://docs.rs/ark-bn254/0.3.0/ark_bn254/), [ark_ec](https://docs.rs/ark-ec/0.3.0/ark_ec/) and [ark_ff](https://docs.rs/ark-ff/0.3.0/ark_ff/).
- The implementation of the poseidon hash is based on [arkworks_gadgets](https://docs.rs/arkworks-gadgets/0.3.14/arkworks_gadgets/poseidon/circom/index.html)
- Light uses a circuit based on [tornado_pool](https://github.com/tornadocash/tornado-pool/tree/onchain-tree/circuits).
