package prover

import (
	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/frontend"
	"github.com/reilabs/gnark-lean-extractor/v2/extractor"
)

func ExtractLean(treeDepth uint32, numberOfCompressedAccounts uint32) (string, error) {
	// Not checking for numberOfCompressedAccounts === 0 or treeDepth === 0

	// Initialising MerkleProofs slice with correct dimentions
	roots := make([]frontend.Variable, numberOfCompressedAccounts)
	leaves := make([]frontend.Variable, numberOfCompressedAccounts)
	inPathIndices := make([]frontend.Variable, numberOfCompressedAccounts)
	inPathElements := make([][]frontend.Variable, numberOfCompressedAccounts)

	for i := 0; i < int(numberOfCompressedAccounts); i++ {
		inPathElements[i] = make([]frontend.Variable, treeDepth)
	}

	inclusionCircuit := InclusionCircuit{
		Depth:          treeDepth,
		NumberOfCompressedAccounts:  numberOfCompressedAccounts,
		Roots:          roots,
		Leaves:         leaves,
		InPathIndices:  inPathIndices,
		InPathElements: inPathElements,
	}

	return extractor.ExtractCircuits("LightProver", ecc.BN254, &inclusionCircuit)
}
