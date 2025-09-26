package v1

import (
	"light/light-prover/prover/common"
	"light/light-prover/prover/poseidon"

	"github.com/consensys/gnark/frontend"
	"github.com/reilabs/gnark-lean-extractor/v3/abstractor"
)

// LegacyLeafHashGadget is for v1 circuits that use 3-input hash with NextIndex
type LegacyLeafHashGadget struct {
	LeafLowerRangeValue  frontend.Variable
	NextIndex            frontend.Variable
	LeafHigherRangeValue frontend.Variable
	Value                frontend.Variable
}

// DefineGadget for LegacyLeafHashGadget - v1 circuits use 3-input hash
func (gadget LegacyLeafHashGadget) DefineGadget(api frontend.API) interface{} {
	// Lower bound is less than value
	abstractor.CallVoid(api, common.AssertIsLess{A: gadget.LeafLowerRangeValue, B: gadget.Value, N: 248})
	// Value is less than upper bound
	abstractor.CallVoid(api, common.AssertIsLess{A: gadget.Value, B: gadget.LeafHigherRangeValue, N: 248})

	return abstractor.Call(api, poseidon.Poseidon3{In1: gadget.LeafLowerRangeValue, In2: gadget.NextIndex, In3: gadget.LeafHigherRangeValue})
}

// LegacyNonInclusionProof is for v1 circuits that use NextIndices
type LegacyNonInclusionProof struct {
	Roots  []frontend.Variable
	Values []frontend.Variable

	LeafLowerRangeValues  []frontend.Variable
	LeafHigherRangeValues []frontend.Variable
	NextIndices           []frontend.Variable

	InPathIndices  []frontend.Variable
	InPathElements [][]frontend.Variable

	NumberOfCompressedAccounts uint32
	Height                     uint32
}

func (gadget LegacyNonInclusionProof) DefineGadget(api frontend.API) interface{} {
	if gadget.NextIndices == nil || len(gadget.NextIndices) == 0 {
		gadget.NextIndices = make([]frontend.Variable, gadget.NumberOfCompressedAccounts)
		for i := 0; i < int(gadget.NumberOfCompressedAccounts); i++ {
			gadget.NextIndices[i] = 0
		}
	}

	currentHash := make([]frontend.Variable, gadget.NumberOfCompressedAccounts)
	for proofIndex := 0; proofIndex < int(gadget.NumberOfCompressedAccounts); proofIndex++ {
		// V1 circuits: use LegacyLeafHashGadget with NextIndex (3-input hash)
		leaf := LegacyLeafHashGadget{
			LeafLowerRangeValue:  gadget.LeafLowerRangeValues[proofIndex],
			NextIndex:            gadget.NextIndices[proofIndex],
			LeafHigherRangeValue: gadget.LeafHigherRangeValues[proofIndex],
			Value:                gadget.Values[proofIndex]}
		currentHash[proofIndex] = abstractor.Call(api, leaf)

		currentPath := api.ToBinary(gadget.InPathIndices[proofIndex], int(gadget.Height))
		hash := common.MerkleRootGadget{
			Hash:   currentHash[proofIndex],
			Index:  currentPath,
			Path:   gadget.InPathElements[proofIndex],
			Height: int(gadget.Height)}
		currentHash[proofIndex] = abstractor.Call(api, hash)
		api.AssertIsEqual(currentHash[proofIndex], gadget.Roots[proofIndex])
	}
	return currentHash
}
