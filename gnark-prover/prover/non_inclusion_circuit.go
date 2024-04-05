package prover

import (
	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
	"github.com/reilabs/gnark-lean-extractor/v2/abstractor"
)

type NonInclusionCircuit struct {
	// public inputs
	Root  []frontend.Variable `gnark:",public"`
	Value []frontend.Variable `gnark:",public"`

	// private inputs
	LeafLowerRangeValue  []frontend.Variable `gnark:"input"`
	LeafHigherRangeValue []frontend.Variable `gnark:"input"`
	LeafIndex            []frontend.Variable `gnark:"input"`

	InPathIndices  []frontend.Variable   `gnark:"input"`
	InPathElements [][]frontend.Variable `gnark:"input"`

	NumberOfUtxos int
	Depth         int
}

func (circuit *NonInclusionCircuit) Define(api frontend.API) error {
	proof := NonInclusionProof{
		Root:  circuit.Root,
		Value: circuit.Value,

		LeafLowerRangeValue:  circuit.LeafLowerRangeValue,
		LeafHigherRangeValue: circuit.LeafHigherRangeValue,
		LeafIndex:            circuit.LeafIndex,

		InPathElements: circuit.InPathElements,
		InPathIndices:  circuit.InPathIndices,

		NumberOfUtxos: circuit.NumberOfUtxos,
		Depth:         circuit.Depth,
	}
	roots := abstractor.Call1(api, proof)

	for i := 0; i < circuit.NumberOfUtxos; i++ {
		api.AssertIsEqual(roots[i], circuit.Root[i])
	}
	return nil
}

func ImportNonInclusionSetup(treeDepth uint32, numberOfUtxos uint32, pkPath string, vkPath string) (*ProvingSystem, error) {
	root := make([]frontend.Variable, numberOfUtxos)
	value := make([]frontend.Variable, numberOfUtxos)

	leafLowerRangeValue := make([]frontend.Variable, numberOfUtxos)
	leafHigherRangeValue := make([]frontend.Variable, numberOfUtxos)
	leafIndex := make([]frontend.Variable, numberOfUtxos)

	inPathIndices := make([]frontend.Variable, numberOfUtxos)
	inPathElements := make([][]frontend.Variable, numberOfUtxos)

	for i := 0; i < int(numberOfUtxos); i++ {
		inPathElements[i] = make([]frontend.Variable, treeDepth)
	}

	circuit := NonInclusionCircuit{
		Depth:                int(treeDepth),
		NumberOfUtxos:        int(numberOfUtxos),
		Root:                 root,
		Value:                value,
		LeafLowerRangeValue:  leafLowerRangeValue,
		LeafHigherRangeValue: leafHigherRangeValue,
		LeafIndex:            leafIndex,
		InPathIndices:        inPathIndices,
		InPathElements:       inPathElements,
	}

	ccs, err := frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &circuit)
	if err != nil {
		return nil, err
	}

	pk, err := LoadProvingKey(pkPath)

	if err != nil {
		return nil, err
	}

	vk, err := LoadVerifyingKey(vkPath)

	if err != nil {
		return nil, err
	}

	return &ProvingSystem{0, 0, treeDepth, numberOfUtxos, pk, vk, ccs}, nil
}
