package prover

import (
	"light/light-prover/prover/poseidon"

	"github.com/consensys/gnark/frontend"
	"github.com/reilabs/gnark-lean-extractor/v3/abstractor"
)

func createHashChain(api frontend.API, length int, hashes []frontend.Variable) frontend.Variable {
	if length == 0 {
		return frontend.Variable(0)
	}

	hashChain := hashes[0]
	for i := 1; i < length; i++ {
		hashChain = abstractor.Call(api, poseidon.Poseidon2{In1: hashChain, In2: hashes[i]})
	}
	return hashChain
}
