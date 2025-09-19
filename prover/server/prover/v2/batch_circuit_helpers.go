package v2

import (
	merkletree "light/light-prover/merkle-tree"
	"light/light-prover/prover/poseidon"
	"math/big"

	"github.com/reilabs/gnark-lean-extractor/v3/abstractor"

	"github.com/consensys/gnark/frontend"
)

type HashChain struct {
	Hashes []frontend.Variable
}

func (gadget HashChain) DefineGadget(api frontend.API) interface{} {
	if len(gadget.Hashes) == 0 {
		return frontend.Variable(0)
	}

	initialHash := gadget.Hashes[0]
	return computeHashChain(api, initialHash, gadget.Hashes)
}

func createHashChain(api frontend.API, hashes []frontend.Variable) frontend.Variable {
	return abstractor.Call(api, HashChain{hashes})
}

type TwoInputsHashChain struct {
	HashesFirst  []frontend.Variable
	HashesSecond []frontend.Variable
}

func (gadget TwoInputsHashChain) DefineGadget(api frontend.API) interface{} {
	if len(gadget.HashesFirst) == 0 {
		panic("HashesFirst must not be empty")
	}

	hashChain := abstractor.Call(api, poseidon.Poseidon2{In1: gadget.HashesFirst[0], In2: gadget.HashesSecond[0]})
	for i := 1; i < len(gadget.HashesFirst); i++ {
		hashChain = abstractor.Call(api, poseidon.Poseidon3{In1: hashChain, In2: gadget.HashesFirst[i], In3: gadget.HashesSecond[i]})
	}
	return hashChain
}

func createTwoInputsHashChain(api frontend.API, hashesFirst []frontend.Variable, hashesSecond []frontend.Variable) frontend.Variable {
	return abstractor.Call(api, TwoInputsHashChain{HashesFirst: hashesFirst, HashesSecond: hashesSecond})
}

func computeHashChain(api frontend.API, initialHash frontend.Variable, hashes []frontend.Variable) frontend.Variable {
	hashChain := initialHash

	for i := 1; i < len(hashes); i++ {
		hashChain = abstractor.Call(api, poseidon.Poseidon2{In1: hashChain, In2: hashes[i]})
	}

	return hashChain
}

// getZeroValue returns the zero value for a given tree level
func getZeroValue(level int) frontend.Variable {
	return frontend.Variable(new(big.Int).SetBytes(merkletree.ZERO_BYTES[level][:]))
}
