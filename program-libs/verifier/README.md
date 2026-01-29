<!-- cargo-rdme start -->

# light-verifier

ZK proof verifier for Light Protocol. Verifies Groth16 proofs
for inclusion, non-inclusion, and combined address+state operations.

| Function | Description |
|----------|-------------|
| [`verify_inclusion_proof`] | Verify inclusion for 1–8+ leaves |
| [`verify_create_addresses_proof`] | Verify non-inclusion for 1–8 addresses |
| [`verify_create_addresses_and_inclusion_proof`] | Verify combined address and state proof |
| [`verify_batch_append_with_proofs`] | Verify batch append (10 or 500 leaves) |
| [`verify_batch_update`] | Verify batch state update (10 or 500) |
| [`verify_batch_address_update`] | Verify batch address update (10 or 250) |
| [`select_verifying_key`] | Route to correct verifying key by leaf/address count |
| [`verify`] | Generic Groth16 proof verification |

<!-- cargo-rdme end -->
