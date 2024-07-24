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
	inclusionInPathElements := make([][]frontend.Variable, numberOfCompressedAccounts)
	nonInclusionInPathElements := make([][]frontend.Variable, numberOfCompressedAccounts)

	for i := 0; i < int(numberOfCompressedAccounts); i++ {
		inclusionInPathElements[i] = make([]frontend.Variable, treeDepth)
		nonInclusionInPathElements[i] = make([]frontend.Variable, treeDepth)
	}

	inclusionCircuit := InclusionCircuit{
		Depth:                      treeDepth,
		NumberOfCompressedAccounts: numberOfCompressedAccounts,
		Roots:                      roots,
		Leaves:                     leaves,
		InPathIndices:              inPathIndices,
		InPathElements:             inclusionInPathElements,
	}

	nonInclusionCircuit := NonInclusionCircuit{
		Depth:                      treeDepth,
		NumberOfCompressedAccounts: numberOfCompressedAccounts,
		Roots:                      roots,
		Values:                     leaves,
		LeafLowerRangeValues:       leaves,
		LeafHigherRangeValues:      leaves,
		NextIndices:                leaves,
		InPathIndices:              inPathIndices,
		InPathElements:             nonInclusionInPathElements,
	}

	combinedCircuit := CombinedCircuit{
		Inclusion:    inclusionCircuit,
		NonInclusion: nonInclusionCircuit,
	}

	return extractor.ExtractCircuits("LightProver", ecc.BN254, &inclusionCircuit, &nonInclusionCircuit, &combinedCircuit)
}
