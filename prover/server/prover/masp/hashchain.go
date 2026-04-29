package masp

import (
	"math/big"

	"light/light-prover/prover/masp/poseidon"
)

// HashChain computes a Poseidon-2 right-fold over inputs:
//
//	result = Poseidon2(inputs[0], Poseidon2(inputs[1], ..., Poseidon2(inputs[N-2], inputs[N-1])))
//
// In iterative form, we walk from the highest index down to 0:
//
//	h = inputs[N-1]
//	for i := N-2; i >= 0; i--:
//	    h = Poseidon2(inputs[i], h)
//
// An empty input returns 0. A single-element input returns that element.
//
// The direction (high → low) makes any suffix of the input list a
// precomputable partial hash: if inputs[k..N-1] are known and stable, the
// intermediate `h` at index k can be cached across many evaluations, and
// only inputs[0..k-1] need to be rehashed per call.
//
// This diverges from light-protocol's create_hash_chain_from_slice, which
// is a left-fold. For a length-2 input the two conventions coincide;
// beyond that they differ.
func HashChain(inputs []*big.Int) *big.Int {
	n := len(inputs)
	if n == 0 {
		return new(big.Int)
	}
	h := new(big.Int).Set(inputs[n-1])
	for i := n - 2; i >= 0; i-- {
		next, err := poseidon.HashWithT(3, []*big.Int{inputs[i], h})
		if err != nil {
			panic(err)
		}
		h = next
	}
	return h
}

// InNullifierChain is a Poseidon2 hash chain over the interleaved
// (in_commit_i, nullifier_i) pairs:
//
//	chain = HashChain([in_commit_0, nullifier_0, in_commit_1, nullifier_1, ...])
//
// Each input has exactly one commitment and one nullifier (1:1 pairing).
// The interleaved form lets an on-chain verifier precompute a table
// `dummy_pair_chain[D]` of "hash of D dummy (commit, nullifier) pairs" and
// fold only the real pairs on top per tx.
func InNullifierChain(inCommits, nullifiers []*big.Int) *big.Int {
	if len(inCommits) != len(nullifiers) {
		panic("masp.InNullifierChain: in_commits and nullifiers must be same length")
	}
	inputs := make([]*big.Int, 0, 2*len(inCommits))
	for i := range inCommits {
		inputs = append(inputs, inCommits[i], nullifiers[i])
	}
	return HashChain(inputs)
}

// TxHash computes the MASP transaction hash with a three-level structure:
//
//	in_nullifier_chain = HashChain([in_commit_0, nullifier_0, in_commit_1, nullifier_1, ...])
//	out_commit_chain   = HashChain([out_commit_0, out_commit_1, ...])
//	tx_hash            = HashChain([1, tx_blinding, in_nullifier_chain, out_commit_chain])
//
// The outer chain has a fixed length (4 elements). The inner chains absorb
// all variable-length content; their dummy tails are what an on-chain
// verifier precomputes.
func TxHash(inCommits, nullifiers, outCommits []*big.Int, txBlinding *big.Int) *big.Int {
	inNullifierChain := InNullifierChain(inCommits, nullifiers)
	outCommitChain := HashChain(outCommits)
	return HashChain([]*big.Int{DomainTxHash, txBlinding, inNullifierChain, outCommitChain})
}
