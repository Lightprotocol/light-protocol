package prover

import (
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/test"
)

func TestBatchAppendWithProofsCircuit(t *testing.T) {
	assert := test.NewAssert(t)

	t.Run("Valid batch update - full HashchainHash", func(t *testing.T) {
		treeDepth := 10
		batchSize := 2
		startIndex := 0
		params := BuildTestBatchAppendWithProofsTree(treeDepth, batchSize, nil, startIndex, false)

		circuit := BatchAppendWithProofsCircuit{
			PublicInputHash:     frontend.Variable(0),
			OldRoot:             frontend.Variable(0),
			NewRoot:             frontend.Variable(0),
			LeavesHashchainHash: frontend.Variable(0),
			OldLeaves:           make([]frontend.Variable, batchSize),
			Leaves:              make([]frontend.Variable, batchSize),
			StartIndex:          frontend.Variable(0),
			MerkleProofs:        make([][]frontend.Variable, batchSize),
			Height:              uint32(treeDepth),
			BatchSize:           uint32(batchSize),
		}

		for i := range circuit.MerkleProofs {
			circuit.MerkleProofs[i] = make([]frontend.Variable, treeDepth)
		}

		witness := BatchAppendWithProofsCircuit{
			PublicInputHash:     frontend.Variable(params.PublicInputHash),
			OldRoot:             frontend.Variable(params.OldRoot),
			NewRoot:             frontend.Variable(params.NewRoot),
			LeavesHashchainHash: frontend.Variable(params.LeavesHashchainHash),
			OldLeaves:           make([]frontend.Variable, batchSize),
			Leaves:              make([]frontend.Variable, batchSize),
			MerkleProofs:        make([][]frontend.Variable, batchSize),
			StartIndex:          frontend.Variable(params.StartIndex),
			Height:              uint32(treeDepth),
			BatchSize:           uint32(batchSize),
		}

		for i := 0; i < batchSize; i++ {
			witness.Leaves[i] = frontend.Variable(params.Leaves[i])
			witness.OldLeaves[i] = frontend.Variable(params.OldLeaves[i])
			witness.StartIndex = frontend.Variable(params.StartIndex)
			witness.MerkleProofs[i] = make([]frontend.Variable, treeDepth)
			for j := 0; j < treeDepth; j++ {
				witness.MerkleProofs[i][j] = frontend.Variable(params.MerkleProofs[i][j])
			}
		}

		err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
		assert.NoError(err)
	})

	t.Run("Mixed batch update", func(t *testing.T) {
		treeDepth := 26
		batchSize := 1000
		startIndex := 0
		enable := true
		params := BuildTestBatchAppendWithProofsTree(treeDepth, batchSize, nil, startIndex, enable)

		circuit := BatchAppendWithProofsCircuit{
			PublicInputHash:     frontend.Variable(0),
			OldRoot:             frontend.Variable(0),
			NewRoot:             frontend.Variable(0),
			LeavesHashchainHash: frontend.Variable(0),
			OldLeaves:           make([]frontend.Variable, batchSize),
			Leaves:              make([]frontend.Variable, batchSize),
			StartIndex:          frontend.Variable(0),
			MerkleProofs:        make([][]frontend.Variable, batchSize),
			Height:              uint32(treeDepth),
			BatchSize:           uint32(batchSize),
		}

		for i := range circuit.MerkleProofs {
			circuit.MerkleProofs[i] = make([]frontend.Variable, treeDepth)
		}

		witness := BatchAppendWithProofsCircuit{
			PublicInputHash:     frontend.Variable(params.PublicInputHash),
			OldRoot:             frontend.Variable(params.OldRoot),
			NewRoot:             frontend.Variable(params.NewRoot),
			LeavesHashchainHash: frontend.Variable(params.LeavesHashchainHash),
			OldLeaves:           make([]frontend.Variable, batchSize),
			Leaves:              make([]frontend.Variable, batchSize),
			MerkleProofs:        make([][]frontend.Variable, batchSize),
			StartIndex:          frontend.Variable(params.StartIndex),
			Height:              uint32(treeDepth),
			BatchSize:           uint32(batchSize),
		}

		for i := 0; i < batchSize; i++ {
			witness.Leaves[i] = frontend.Variable(params.Leaves[i])
			witness.OldLeaves[i] = frontend.Variable(params.OldLeaves[i])
			witness.StartIndex = frontend.Variable(params.StartIndex)
			witness.MerkleProofs[i] = make([]frontend.Variable, treeDepth)
			for j := 0; j < treeDepth; j++ {
				witness.MerkleProofs[i][j] = frontend.Variable(params.MerkleProofs[i][j])
			}
		}

		err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
		assert.NoError(err)
	})
}
