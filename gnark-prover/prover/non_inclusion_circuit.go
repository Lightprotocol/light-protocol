package prover

import (
	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
	"github.com/reilabs/gnark-lean-extractor/v2/abstractor"
)

type NonInclusionCircuit struct {
	// public inputs
	Roots  []frontend.Variable `gnark:",public"`
	Values []frontend.Variable `gnark:",public"`

	// private inputs
	LeafLowerRangeValues  []frontend.Variable `gnark:"input"`
	LeafHigherRangeValues []frontend.Variable `gnark:"input"`
	LeafIndices           []frontend.Variable `gnark:"input"`

	InPathIndices  []frontend.Variable   `gnark:"input"`
	InPathElements [][]frontend.Variable `gnark:"input"`

	NumberOfUtxos int
	Depth         int
}

func (circuit *NonInclusionCircuit) Define(api frontend.API) error {
	proof := NonInclusionProof{
		Roots:  circuit.Roots,
		Values: circuit.Values,

		LeafLowerRangeValues:  circuit.LeafLowerRangeValues,
		LeafHigherRangeValues: circuit.LeafHigherRangeValues,
		LeafIndices:           circuit.LeafIndices,

		InPathElements: circuit.InPathElements,
		InPathIndices:  circuit.InPathIndices,

		NumberOfUtxos: circuit.NumberOfUtxos,
		Depth:         circuit.Depth,
	}
	roots := abstractor.Call1(api, proof)

	for i := 0; i < circuit.NumberOfUtxos; i++ {
		api.AssertIsEqual(roots[i], circuit.Roots[i])
	}
	return nil
}

func ImportNonInclusionSetup(treeDepth uint32, numberOfUtxos uint32, pkPath string, vkPath string) (*ProvingSystem, error) {
	roots := make([]frontend.Variable, numberOfUtxos)
	values := make([]frontend.Variable, numberOfUtxos)

	leafLowerRangeValues := make([]frontend.Variable, numberOfUtxos)
	leafHigherRangeValues := make([]frontend.Variable, numberOfUtxos)
	leafIndices := make([]frontend.Variable, numberOfUtxos)

	inPathIndices := make([]frontend.Variable, numberOfUtxos)
	inPathElements := make([][]frontend.Variable, numberOfUtxos)

	for i := 0; i < int(numberOfUtxos); i++ {
		inPathElements[i] = make([]frontend.Variable, treeDepth)
	}

	circuit := NonInclusionCircuit{
		Depth:                 int(treeDepth),
		NumberOfUtxos:         int(numberOfUtxos),
		Roots:                 roots,
		Values:                values,
		LeafLowerRangeValues:  leafLowerRangeValues,
		LeafHigherRangeValues: leafHigherRangeValues,
		LeafIndices:           leafIndices,
		InPathIndices:         inPathIndices,
		InPathElements:        inPathElements,
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
