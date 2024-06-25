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
	NextIndices           []frontend.Variable `gnark:"input"`

	InPathIndices  []frontend.Variable   `gnark:"input"`
	InPathElements [][]frontend.Variable `gnark:"input"`

	NumberOfCompressedAccounts uint32
	Depth                      uint32
}

func (circuit *NonInclusionCircuit) Define(api frontend.API) error {
	proof := NonInclusionProof{
		Roots:  circuit.Roots,
		Values: circuit.Values,

		LeafLowerRangeValues:  circuit.LeafLowerRangeValues,
		LeafHigherRangeValues: circuit.LeafHigherRangeValues,
		NextIndices:           circuit.NextIndices,

		InPathElements: circuit.InPathElements,
		InPathIndices:  circuit.InPathIndices,

		NumberOfCompressedAccounts: circuit.NumberOfCompressedAccounts,
		Depth:                      circuit.Depth,
	}
	roots := abstractor.Call1(api, proof)
	for i := 0; i < int(circuit.NumberOfCompressedAccounts); i++ {
		api.AssertIsEqual(roots[i], circuit.Roots[i])
	}
	return nil
}

func ImportNonInclusionSetup(treeDepth uint32, numberOfCompressedAccounts uint32, pkPath string, vkPath string) (*ProvingSystem, error) {
	roots := make([]frontend.Variable, numberOfCompressedAccounts)
	values := make([]frontend.Variable, numberOfCompressedAccounts)

	leafLowerRangeValues := make([]frontend.Variable, numberOfCompressedAccounts)
	leafHigherRangeValues := make([]frontend.Variable, numberOfCompressedAccounts)
	leafIndices := make([]frontend.Variable, numberOfCompressedAccounts)

	inPathIndices := make([]frontend.Variable, numberOfCompressedAccounts)
	inPathElements := make([][]frontend.Variable, numberOfCompressedAccounts)

	for i := 0; i < int(numberOfCompressedAccounts); i++ {
		inPathElements[i] = make([]frontend.Variable, treeDepth)
	}

	circuit := NonInclusionCircuit{
		Depth:                      treeDepth,
		NumberOfCompressedAccounts: numberOfCompressedAccounts,
		Roots:                      roots,
		Values:                     values,
		LeafLowerRangeValues:       leafLowerRangeValues,
		LeafHigherRangeValues:      leafHigherRangeValues,
		NextIndices:                leafIndices,
		InPathIndices:              inPathIndices,
		InPathElements:             inPathElements,
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

	return &ProvingSystem{0, 0, treeDepth, numberOfCompressedAccounts, pk, vk, ccs}, nil
}
