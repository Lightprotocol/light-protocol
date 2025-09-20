package v2

import (
	"math/big"
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/test"
)

func TestBatchAppendCircuit(t *testing.T) {
	assert := test.NewAssert(t)

	t.Run("Valid batch update - full HashchainHash", func(t *testing.T) {
		treeDepth := 10
		batchSize := 2
		startIndex := 0
		params := BuildTestBatchAppendTree(treeDepth, batchSize, nil, startIndex, false)

		circuit := BatchAppendCircuit{
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

		witness := BatchAppendCircuit{
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
		params := BuildTestBatchAppendTree(treeDepth, batchSize, nil, startIndex, enable)

		circuit := BatchAppendCircuit{
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

		witness := BatchAppendCircuit{
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

	t.Run("Invalid public input hash", func(t *testing.T) {
		treeDepth := 10
		batchSize := 2
		params := BuildTestBatchAppendTree(treeDepth, batchSize, nil, 0, false)
		params.PublicInputHash = big.NewInt(999)

		witness := createTestWitness(*params)
		circuit := createTestCircuit(treeDepth, batchSize)

		err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
		assert.Error(err)
	})

	t.Run("Invalid old root", func(t *testing.T) {
		treeDepth := 10
		batchSize := 2
		params := BuildTestBatchAppendTree(treeDepth, batchSize, nil, 0, false)
		params.OldRoot = big.NewInt(999)

		witness := createTestWitness(*params)
		circuit := createTestCircuit(treeDepth, batchSize)

		err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
		assert.Error(err)
	})

	t.Run("Invalid new root", func(t *testing.T) {
		treeDepth := 10
		batchSize := 2
		params := BuildTestBatchAppendTree(treeDepth, batchSize, nil, 0, false)
		params.NewRoot = big.NewInt(999)

		witness := createTestWitness(*params)
		circuit := createTestCircuit(treeDepth, batchSize)

		err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
		assert.Error(err)
	})

	t.Run("Invalid leaves hashchain", func(t *testing.T) {
		treeDepth := 10
		batchSize := 2
		params := BuildTestBatchAppendTree(treeDepth, batchSize, nil, 0, false)
		params.LeavesHashchainHash = big.NewInt(999)

		witness := createTestWitness(*params)
		circuit := createTestCircuit(treeDepth, batchSize)

		err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
		assert.Error(err)
	})

	t.Run("Invalid merkle proof", func(t *testing.T) {
		treeDepth := 10
		batchSize := 2
		params := BuildTestBatchAppendTree(treeDepth, batchSize, nil, 0, false)
		params.MerkleProofs[0][0] = *big.NewInt(999)

		witness := createTestWitness(*params)
		circuit := createTestCircuit(treeDepth, batchSize)

		err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
		assert.Error(err)
	})

	t.Run("Invalid start index", func(t *testing.T) {
		treeDepth := 10
		batchSize := 2
		params := BuildTestBatchAppendTree(treeDepth, batchSize, nil, 0, false)
		params.StartIndex = uint64(1 << treeDepth)

		witness := createTestWitness(*params)
		circuit := createTestCircuit(treeDepth, batchSize)

		err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
		assert.Error(err)
	})

	t.Run("Invalid old leaves", func(t *testing.T) {
		assert := test.NewAssert(t)
		treeDepth := 10
		batchSize := 2
		params := BuildTestBatchAppendTree(treeDepth, batchSize, nil, 0, false)

		params.OldLeaves[0] = big.NewInt(999)

		witness := createTestWitness(*params)
		circuit := createTestCircuit(treeDepth, batchSize)

		err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
		assert.Error(err)
	})

	t.Run("Invalid leaves", func(t *testing.T) {
		assert := test.NewAssert(t)
		treeDepth := 10
		batchSize := 2
		params := BuildTestBatchAppendTree(treeDepth, batchSize, nil, 0, false)

		params.Leaves[0] = big.NewInt(999)

		witness := createTestWitness(*params)
		circuit := createTestCircuit(treeDepth, batchSize)

		err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
		assert.Error(err)
	})
}

func createTestCircuit(treeDepth, batchSize int) BatchAppendCircuit {
	circuit := BatchAppendCircuit{
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
	return circuit
}

func createTestWitness(params BatchAppendParameters) BatchAppendCircuit {
	witness := BatchAppendCircuit{
		PublicInputHash:     frontend.Variable(params.PublicInputHash),
		OldRoot:             frontend.Variable(params.OldRoot),
		NewRoot:             frontend.Variable(params.NewRoot),
		LeavesHashchainHash: frontend.Variable(params.LeavesHashchainHash),
		OldLeaves:           make([]frontend.Variable, int(params.BatchSize)),
		Leaves:              make([]frontend.Variable, int(params.BatchSize)),
		MerkleProofs:        make([][]frontend.Variable, int(params.BatchSize)),
		StartIndex:          frontend.Variable(params.StartIndex),
		Height:              params.Height,
		BatchSize:           params.BatchSize,
	}

	for i := 0; i < int(params.BatchSize); i++ {
		witness.Leaves[i] = frontend.Variable(params.Leaves[i])
		witness.OldLeaves[i] = frontend.Variable(params.OldLeaves[i])
		witness.MerkleProofs[i] = make([]frontend.Variable, params.Height)
		for j := 0; j < int(params.Height); j++ {
			witness.MerkleProofs[i][j] = frontend.Variable(params.MerkleProofs[i][j])
		}
	}
	return witness
}
