package masp

import (
	"math/big"

	"light/light-prover/prover/masp/poseidon"
)

// KeypairOwner computes the BN254-Fr owner value for a P-256 public key. The
// 256-bit X and Y coordinates are each split into two 128-bit halves (low,
// high) and hashed with Poseidon4(X_hi, X_lo, Y_hi, Y_lo). This matches the
// in-circuit derivation in keypairOwnerCircuit.
func KeypairOwner(pubX, pubY *big.Int) *big.Int {
	xHi, xLo := splitHalves128(pubX)
	yHi, yLo := splitHalves128(pubY)
	h, err := poseidon.HashWithT(5, []*big.Int{xHi, xLo, yHi, yLo})
	if err != nil {
		panic(err)
	}
	return h
}

// splitHalves128 splits a non-negative big.Int of up to 256 bits into its
// high 128 bits and low 128 bits.
func splitHalves128(v *big.Int) (hi, lo *big.Int) {
	mask128, _ := new(big.Int).SetString("ffffffffffffffffffffffffffffffff", 16)
	lo = new(big.Int).And(v, mask128)
	hi = new(big.Int).Rsh(v, 128)
	return hi, lo
}
