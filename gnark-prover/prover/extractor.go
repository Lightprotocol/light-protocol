package prover

import (
	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/frontend"
	"github.com/reilabs/gnark-lean-extractor/v2/extractor"
)

func ExtractLean(treeDepth uint32, numberOfUtxos uint32) (string, error) {
	// Not checking for numberOfUtxos === 0 or treeDepth === 0

	// Initialising MerkleProofs slice with correct dimentions
	roots := make([]frontend.Variable, numberOfUtxos)
	leaves := make([]frontend.Variable, numberOfUtxos)
	inPathIndices := make([]frontend.Variable, numberOfUtxos)
	inPathElements := make([][]frontend.Variable, numberOfUtxos)

	for i := 0; i < int(numberOfUtxos); i++ {
		inPathElements[i] = make([]frontend.Variable, treeDepth)
	}

	inclusionCircuit := InclusionCircuit{
		Depth:          int(treeDepth),
		NumberOfUtxos:  int(numberOfUtxos),
		Roots:          roots,
		Leaves:         leaves,
		InPathIndices:  inPathIndices,
		InPathElements: inPathElements,
	}

	return extractor.ExtractCircuits("LightProver", ecc.BN254, &inclusionCircuit)
}
