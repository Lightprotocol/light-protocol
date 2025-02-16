package prover

import (
	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/frontend"
	"github.com/reilabs/gnark-lean-extractor/v2/extractor"
)

func ExtractLean(treeHeight uint32, numberOfCompressedAccounts uint32) (string, error) {
	// Not checking for numberOfCompressedAccounts === 0 or treeHeight === 0

	// Initialising MerkleProofs slice with correct dimensions
	inclusionInPathElements := make([][]frontend.Variable, numberOfCompressedAccounts)
	nonInclusionInPathElements := make([][]frontend.Variable, numberOfCompressedAccounts)
	addressAppendLowProofs := make([][]frontend.Variable, numberOfCompressedAccounts)
	addressAppendEmptyProofs := make([][]frontend.Variable, numberOfCompressedAccounts)
	batchUpdateProofs := make([][]frontend.Variable, numberOfCompressedAccounts)
	batchUpdateWithProofsProofs := make([][]frontend.Variable, numberOfCompressedAccounts)

	for i := 0; i < int(numberOfCompressedAccounts); i++ {
		inclusionInPathElements[i] = make([]frontend.Variable, treeHeight)
		nonInclusionInPathElements[i] = make([]frontend.Variable, treeHeight)
		addressAppendLowProofs[i] = make([]frontend.Variable, treeHeight)
		addressAppendEmptyProofs[i] = make([]frontend.Variable, treeHeight)
		batchUpdateProofs[i] = make([]frontend.Variable, treeHeight)
		batchUpdateWithProofsProofs[i] = make([]frontend.Variable, treeHeight)
	}

	inclusionCircuit := InclusionCircuit{
		Height:                     treeHeight,
		NumberOfCompressedAccounts: numberOfCompressedAccounts,
		Roots:                      make([]frontend.Variable, numberOfCompressedAccounts),
		Leaves:                     make([]frontend.Variable, numberOfCompressedAccounts),
		InPathIndices:              make([]frontend.Variable, numberOfCompressedAccounts),
		InPathElements:             inclusionInPathElements,
	}

	nonInclusionCircuit := NonInclusionCircuit{
		Height:                     treeHeight,
		NumberOfCompressedAccounts: numberOfCompressedAccounts,
		Roots:                      make([]frontend.Variable, numberOfCompressedAccounts),
		Values:                     make([]frontend.Variable, numberOfCompressedAccounts),
		LeafLowerRangeValues:       make([]frontend.Variable, numberOfCompressedAccounts),
		LeafHigherRangeValues:      make([]frontend.Variable, numberOfCompressedAccounts),
		InPathIndices:              make([]frontend.Variable, numberOfCompressedAccounts),
		InPathElements:             nonInclusionInPathElements,
	}
	inclusionProof := InclusionProof{
		Height:                     treeHeight,
		NumberOfCompressedAccounts: numberOfCompressedAccounts,
		Roots:                      make([]frontend.Variable, numberOfCompressedAccounts),
		Leaves:                     make([]frontend.Variable, numberOfCompressedAccounts),
		InPathIndices:              make([]frontend.Variable, numberOfCompressedAccounts),
		InPathElements:             inclusionInPathElements,
	}

	nonInclusionProof := NonInclusionProof{
		Height:                     treeHeight,
		NumberOfCompressedAccounts: numberOfCompressedAccounts,
		Roots:                      make([]frontend.Variable, numberOfCompressedAccounts),
		Values:                     make([]frontend.Variable, numberOfCompressedAccounts),
		LeafLowerRangeValues:       make([]frontend.Variable, numberOfCompressedAccounts),
		LeafHigherRangeValues:      make([]frontend.Variable, numberOfCompressedAccounts),
		InPathIndices:              make([]frontend.Variable, numberOfCompressedAccounts),
		InPathElements:             nonInclusionInPathElements,
	}

	combinedCircuit := CombinedCircuit{
		Inclusion:    inclusionProof,
		NonInclusion: nonInclusionProof,
	}

	indexedUpdateCircuit := BatchAddressTreeAppendCircuit{
		LowElementValues:     make([]frontend.Variable, numberOfCompressedAccounts),
		LowElementNextValues: make([]frontend.Variable, numberOfCompressedAccounts),
		LowElementIndices:    make([]frontend.Variable, numberOfCompressedAccounts),
		LowElementProofs:     addressAppendLowProofs,
		NewElementValues:     make([]frontend.Variable, numberOfCompressedAccounts),
		NewElementProofs:     addressAppendEmptyProofs,
		BatchSize:            numberOfCompressedAccounts,
		TreeHeight:           treeHeight,
	}

	batchUpdateCircuit := BatchUpdateCircuit{
		TxHashes:     make([]frontend.Variable, numberOfCompressedAccounts),
		Leaves:       make([]frontend.Variable, numberOfCompressedAccounts),
		OldLeaves:    make([]frontend.Variable, numberOfCompressedAccounts),
		MerkleProofs: batchUpdateProofs,
		PathIndices:  make([]frontend.Variable, numberOfCompressedAccounts),
		Height:       treeHeight,
		BatchSize:    numberOfCompressedAccounts,
	}

	batchAppendWithProofsCircuit := BatchAppendWithProofsCircuit{
		OldLeaves:    make([]frontend.Variable, numberOfCompressedAccounts),
		Leaves:       make([]frontend.Variable, numberOfCompressedAccounts),
		MerkleProofs: batchUpdateWithProofsProofs,
		Height:       treeHeight,
		BatchSize:    numberOfCompressedAccounts,
	}

	return extractor.ExtractCircuits(
		"LightProver",
		ecc.BN254,
		&inclusionCircuit,
		&nonInclusionCircuit,
		&combinedCircuit,
		&batchUpdateCircuit,
		&indexedUpdateCircuit,
		&batchAppendWithProofsCircuit,
	)
}
