package masp

import (
	"math/big"

	"light/light-prover/prover/masp/poseidon"
)

// DomainUtxo is the domain tag baked into UtxoHash as its first field input.
var DomainUtxo = big.NewInt(0)

// DomainTxHash is the initial accumulator of the tx_hash HashChain.
var DomainTxHash = big.NewInt(1)

// Utxo is the field-element view of a MASP UTXO.
type Utxo struct {
	Owner    *big.Int
	Spl      *big.Int
	Sol      *big.Int
	Blinding *big.Int
	DataHash *big.Int
}

// UtxoHash computes Poseidon6(0, owner, spl, sol, blinding, data_hash) — the
// MASP UTXO commitment. Width t=7; the leading 0 is the literal
// user-supplied domain tag (distinct from Circom's internal zero domain tag
// that occupies state position 0).
func UtxoHash(u Utxo) *big.Int {
	h, err := poseidon.HashWithT(7, []*big.Int{
		DomainUtxo,
		u.Owner,
		u.Spl,
		u.Sol,
		u.Blinding,
		u.DataHash,
	})
	if err != nil {
		panic(err)
	}
	return h
}
