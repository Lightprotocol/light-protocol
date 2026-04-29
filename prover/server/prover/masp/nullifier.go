package masp

import (
	"math/big"

	"light/light-prover/prover/masp/poseidon"
)

// DomainNullifierSecret derives the per-commitment spending capability:
//
//	dns = Poseidon2(in_commit, source)
//
// For keypair-owned UTXOs (seed == 0) `source` is the owner's master
// nullifier_secret — the secret never leaves the user's device; only dns
// reaches the backend prover (tree_circuit).
// For program-owned UTXOs (seed != 0) `source` is the program_id. The user's
// master secret is never mixed into program-owned nullifiers; the program's
// spending authority is its ability to make a CPI with the matching
// program_id, not knowledge of a user secret.
func DomainNullifierSecret(inCommit, source *big.Int) *big.Int {
	h, err := poseidon.HashWithT(3, []*big.Int{inCommit, source})
	if err != nil {
		panic(err)
	}
	return h
}

// Nullify is the public nullifier of a UTXO being spent:
//
//	nullifier_i = Poseidon3(in_commit_i, leaf_index_i, dns_i)
func Nullify(inCommit, leafIndex, domainSecret *big.Int) *big.Int {
	h, err := poseidon.HashWithT(4, []*big.Int{inCommit, leafIndex, domainSecret})
	if err != nil {
		panic(err)
	}
	return h
}
