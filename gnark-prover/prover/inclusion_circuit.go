package prover

import (
	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
	"github.com/reilabs/gnark-lean-extractor/v2/abstractor"
)

type InclusionCircuit struct {
	// public inputs
	Roots  []frontend.Variable `gnark:",public"`
	Leaves []frontend.Variable `gnark:",public"`

	// private inputs
	InPathIndices  []frontend.Variable   `gnark:"input"`
	InPathElements [][]frontend.Variable `gnark:"input"`

	NumberOfUtxos int
	Depth         int
}

func (circuit *InclusionCircuit) Define(api frontend.API) error {
	// Actual merkle proof verification.
	abstractor.Call1(api, InclusionProof{
		Roots:          circuit.Roots,
		Leaves:         circuit.Leaves,
		InPathElements: circuit.InPathElements,
		InPathIndices:  circuit.InPathIndices,

		NumberOfUtxos: circuit.NumberOfUtxos,
		Depth:         circuit.Depth,
	})
	return nil
}

func ImportInclusionSetup(treeDepth uint32, numberOfUtxos uint32, pkPath string, vkPath string) (*ProvingSystem, error) {
	roots := make([]frontend.Variable, numberOfUtxos)
	leaves := make([]frontend.Variable, numberOfUtxos)
	inPathIndices := make([]frontend.Variable, numberOfUtxos)
	inPathElements := make([][]frontend.Variable, numberOfUtxos)

	circuit := InclusionCircuit{
		Depth:          int(treeDepth),
		NumberOfUtxos:  int(numberOfUtxos),
		Roots:          roots,
		Leaves:         leaves,
		InPathIndices:  inPathIndices,
		InPathElements: inPathElements,
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

	return &ProvingSystem{treeDepth, numberOfUtxos, 0, 0, pk, vk, ccs}, nil
}
