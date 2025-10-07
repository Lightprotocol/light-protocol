package v2

import (
	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/frontend"
	"github.com/reilabs/gnark-lean-extractor/v3/extractor"
	"light/light-prover/prover/common"
)

func ExtractLean(stateTreeHeight uint32, addressTreeHeight uint32, numberOfCompressedAccounts uint32) (string, error) {
	// Not checking for numberOfCompressedAccounts === 0 or treeHeight === 0

	// Initialising MerkleProofs slice with correct dimensions
	inclusionInPathElements := make([][]frontend.Variable, numberOfCompressedAccounts)
	nonInclusionInPathElements := make([][]frontend.Variable, numberOfCompressedAccounts)
	addressAppendLowProofs := make([][]frontend.Variable, numberOfCompressedAccounts)
	addressAppendEmptyProofs := make([][]frontend.Variable, numberOfCompressedAccounts)
	batchUpdateProofs := make([][]frontend.Variable, numberOfCompressedAccounts)
	batchUpdateWithProofsProofs := make([][]frontend.Variable, numberOfCompressedAccounts)

	for i := 0; i < int(numberOfCompressedAccounts); i++ {
		inclusionInPathElements[i] = make([]frontend.Variable, stateTreeHeight)
		nonInclusionInPathElements[i] = make([]frontend.Variable, addressTreeHeight)
		addressAppendLowProofs[i] = make([]frontend.Variable, addressTreeHeight)
		addressAppendEmptyProofs[i] = make([]frontend.Variable, addressTreeHeight)
		batchUpdateProofs[i] = make([]frontend.Variable, stateTreeHeight)
		batchUpdateWithProofsProofs[i] = make([]frontend.Variable, stateTreeHeight)
	}

	inclusionCircuit := InclusionCircuit{
		Height:                     stateTreeHeight,
		NumberOfCompressedAccounts: numberOfCompressedAccounts,
		Roots:                      make([]frontend.Variable, numberOfCompressedAccounts),
		Leaves:                     make([]frontend.Variable, numberOfCompressedAccounts),
		InPathIndices:              make([]frontend.Variable, numberOfCompressedAccounts),
		InPathElements:             inclusionInPathElements,
	}

	nonInclusionCircuit := NonInclusionCircuit{
		Height:                     addressTreeHeight,
		NumberOfCompressedAccounts: numberOfCompressedAccounts,
		Roots:                      make([]frontend.Variable, numberOfCompressedAccounts),
		Values:                     make([]frontend.Variable, numberOfCompressedAccounts),
		LeafLowerRangeValues:       make([]frontend.Variable, numberOfCompressedAccounts),
		LeafHigherRangeValues:      make([]frontend.Variable, numberOfCompressedAccounts),
		InPathIndices:              make([]frontend.Variable, numberOfCompressedAccounts),
		InPathElements:             nonInclusionInPathElements,
	}
	inclusionProof := common.InclusionProof{
		Height:                     stateTreeHeight,
		NumberOfCompressedAccounts: numberOfCompressedAccounts,
		Roots:                      make([]frontend.Variable, numberOfCompressedAccounts),
		Leaves:                     make([]frontend.Variable, numberOfCompressedAccounts),
		InPathIndices:              make([]frontend.Variable, numberOfCompressedAccounts),
		InPathElements:             inclusionInPathElements,
	}

	nonInclusionProof := common.NonInclusionProof{
		Height:                     addressTreeHeight,
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
		TreeHeight:           addressTreeHeight,
	}

	batchUpdateCircuit := BatchUpdateCircuit{
		TxHashes:     make([]frontend.Variable, numberOfCompressedAccounts),
		Leaves:       make([]frontend.Variable, numberOfCompressedAccounts),
		OldLeaves:    make([]frontend.Variable, numberOfCompressedAccounts),
		MerkleProofs: batchUpdateProofs,
		PathIndices:  make([]frontend.Variable, numberOfCompressedAccounts),
		Height:       stateTreeHeight,
		BatchSize:    numberOfCompressedAccounts,
	}

	batchAppendCircuit := BatchAppendCircuit{
		OldLeaves:    make([]frontend.Variable, numberOfCompressedAccounts),
		Leaves:       make([]frontend.Variable, numberOfCompressedAccounts),
		MerkleProofs: batchUpdateWithProofsProofs,
		Height:       stateTreeHeight,
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
		&batchAppendCircuit,
	)
}
