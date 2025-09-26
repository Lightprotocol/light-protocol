package v2

import (
	"fmt"
	merkletree "light/light-prover/merkle-tree"
	"math/big"
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/test"
)

func TestBatchUpdateCircuit(t *testing.T) {
	assert := test.NewAssert(t)

	t.Run("Valid batch update - full HashchainHash", func(t *testing.T) {
		treeDepth := 10
		batchSize := 2
		params := BuildTestBatchUpdateTree(treeDepth, batchSize, nil, nil)

		circuit := BatchUpdateCircuit{
			PublicInputHash:     frontend.Variable(0),
			OldRoot:             frontend.Variable(0),
			NewRoot:             frontend.Variable(0),
			LeavesHashchainHash: frontend.Variable(0),
			TxHashes:            make([]frontend.Variable, batchSize),
			Leaves:              make([]frontend.Variable, batchSize),
			OldLeaves:           make([]frontend.Variable, batchSize),
			PathIndices:         make([]frontend.Variable, batchSize),
			MerkleProofs:        make([][]frontend.Variable, batchSize),
			Height:              uint32(treeDepth),
			BatchSize:           uint32(batchSize),
		}

		for i := range circuit.MerkleProofs {
			circuit.MerkleProofs[i] = make([]frontend.Variable, treeDepth)
		}

		witness := BatchUpdateCircuit{
			PublicInputHash:     frontend.Variable(params.PublicInputHash),
			OldRoot:             frontend.Variable(params.OldRoot),
			NewRoot:             frontend.Variable(params.NewRoot),
			LeavesHashchainHash: frontend.Variable(params.LeavesHashchainHash),
			TxHashes:            make([]frontend.Variable, batchSize),
			Leaves:              make([]frontend.Variable, batchSize),
			OldLeaves:           make([]frontend.Variable, batchSize),
			MerkleProofs:        make([][]frontend.Variable, batchSize),
			PathIndices:         make([]frontend.Variable, batchSize),
			Height:              uint32(treeDepth),
			BatchSize:           uint32(batchSize),
		}

		for i := 0; i < batchSize; i++ {
			witness.Leaves[i] = frontend.Variable(params.Leaves[i])
			witness.OldLeaves[i] = frontend.Variable(params.OldLeaves[i])
			witness.TxHashes[i] = frontend.Variable(params.TxHashes[i])
			witness.PathIndices[i] = frontend.Variable(params.PathIndices[i])
			witness.MerkleProofs[i] = make([]frontend.Variable, treeDepth)
			for j := 0; j < treeDepth; j++ {
				witness.MerkleProofs[i][j] = frontend.Variable(params.MerkleProofs[i][j])
			}
		}

		err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
		assert.NoError(err)
	})

	t.Run("Fill up tree completely", func(t *testing.T) {
		treeDepth := 8
		batchSize := 4
		totalLeaves := 1 << treeDepth

		var tree = merkletree.NewTree(int(treeDepth))
		for i := 0; i < totalLeaves/batchSize; i++ {
			startIndex := uint32(i * batchSize)
			params := BuildTestBatchUpdateTree(treeDepth, batchSize, &tree, &startIndex)

			circuit := createBatchUpdateCircuit(treeDepth, batchSize)
			witness := createBatchUpdateWitness(*params, 0, batchSize)

			err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
			assert.NoError(err)
			tree = *params.Tree.DeepCopy()
		}
	})

	t.Run("Different tree depths and batch sizes", func(t *testing.T) {
		testCases := []struct {
			treeDepth int
			batchSize int
		}{
			{4, 1},
			{10, 100},
			{26, 10},
		}

		for _, tc := range testCases {
			t.Run(fmt.Sprintf("Depth:%d_Batch:%d", tc.treeDepth, tc.batchSize), func(t *testing.T) {
				params := BuildTestBatchUpdateTree(tc.treeDepth, tc.batchSize, nil, nil)
				circuit := createBatchUpdateCircuit(tc.treeDepth, tc.batchSize)
				witness := createBatchUpdateWitness(*params, 0, tc.batchSize)

				err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
				assert.NoError(err)
			})
		}
	})

	t.Run("Invalid OldRoot", func(t *testing.T) {
		treeDepth := 10
		batchSize := 5
		params := BuildTestBatchUpdateTree(treeDepth, batchSize, nil, nil)

		circuit := createBatchUpdateCircuit(treeDepth, batchSize)
		witness := createBatchUpdateWitness(*params, 0, batchSize)

		witness.OldRoot = frontend.Variable(new(big.Int).Add(params.OldRoot, big.NewInt(1)))

		err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
		assert.Error(err)
	})

	t.Run("Invalid NewRoot", func(t *testing.T) {
		treeDepth := 10
		batchSize := 5
		params := BuildTestBatchUpdateTree(treeDepth, batchSize, nil, nil)

		circuit := createBatchUpdateCircuit(treeDepth, batchSize)
		witness := createBatchUpdateWitness(*params, 0, batchSize)

		// Modify NewRoot to make it invalid
		witness.NewRoot = frontend.Variable(new(big.Int).Add(params.NewRoot, big.NewInt(1)))

		err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
		assert.Error(err)
	})

	t.Run("Invalid old leaf", func(t *testing.T) {
		treeDepth := 10
		batchSize := 5
		params := BuildTestBatchUpdateTree(treeDepth, batchSize, nil, nil)

		circuit := createBatchUpdateCircuit(treeDepth, batchSize)
		witness := createBatchUpdateWitness(*params, 0, batchSize)

		// Modify one old leaf to make it invalid
		witness.OldLeaves[0] = frontend.Variable(new(big.Int).Add(params.OldLeaves[0], big.NewInt(1)))

		err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
		assert.Error(err)
	})

	t.Run("Invalid PublicInputHash", func(t *testing.T) {
		treeDepth := 10
		batchSize := 5
		params := BuildTestBatchUpdateTree(treeDepth, batchSize, nil, nil)

		circuit := createBatchUpdateCircuit(treeDepth, batchSize)
		witness := createBatchUpdateWitness(*params, 0, batchSize)

		witness.PublicInputHash = frontend.Variable(new(big.Int).Add(params.PublicInputHash, big.NewInt(1)))

		err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
		assert.Error(err)
	})

	t.Run("Invalid PathIndex", func(t *testing.T) {
		treeDepth := 10
		batchSize := 5
		params := BuildTestBatchUpdateTree(treeDepth, batchSize, nil, nil)

		circuit := createBatchUpdateCircuit(treeDepth, batchSize)
		witness := createBatchUpdateWitness(*params, 0, batchSize)

		// Set invalid path index
		witness.PathIndices[0] = frontend.Variable(uint32(1 << treeDepth))

		err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
		assert.Error(err)
	})

	t.Run("Invalid MerkleProof", func(t *testing.T) {
		treeDepth := 10
		batchSize := 5
		params := BuildTestBatchUpdateTree(treeDepth, batchSize, nil, nil)

		circuit := createBatchUpdateCircuit(treeDepth, batchSize)
		witness := createBatchUpdateWitness(*params, 0, batchSize)

		// Corrupt merkle proof
		witness.MerkleProofs[0][0] = frontend.Variable(new(big.Int).Add(big.NewInt(0).SetBytes(params.MerkleProofs[0][0].Bytes()), big.NewInt(1)))

		err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
		assert.Error(err)
	})

	t.Run("Invalid LeavesHashchainHash", func(t *testing.T) {
		treeDepth := 10
		batchSize := 5
		params := BuildTestBatchUpdateTree(treeDepth, batchSize, nil, nil)

		circuit := createBatchUpdateCircuit(treeDepth, batchSize)
		witness := createBatchUpdateWitness(*params, 0, batchSize)

		// Modify LeavesHashchainHash to make it invalid
		witness.LeavesHashchainHash = frontend.Variable(new(big.Int).Add(params.LeavesHashchainHash, big.NewInt(1)))

		err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
		assert.Error(err)
	})

	t.Run("Invalid leaf", func(t *testing.T) {
		treeDepth := 10
		batchSize := 5
		params := BuildTestBatchUpdateTree(treeDepth, batchSize, nil, nil)

		circuit := createBatchUpdateCircuit(treeDepth, batchSize)
		witness := createBatchUpdateWitness(*params, 0, batchSize)

		// Modify one leaf to make it invalid
		witness.Leaves[0] = frontend.Variable(new(big.Int).Add(params.Leaves[0], big.NewInt(1)))

		err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
		assert.Error(err)
	})

	t.Run("Invalid order of leaves", func(t *testing.T) {
		treeDepth := 10
		batchSize := 5
		params := BuildTestBatchUpdateTree(treeDepth, batchSize, nil, nil)

		circuit := createBatchUpdateCircuit(treeDepth, batchSize)
		witness := createBatchUpdateWitness(*params, 0, batchSize)

		// Swap two leaves to create an invalid order
		witness.Leaves[0], witness.Leaves[1] = witness.Leaves[1], witness.Leaves[0]

		err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
		assert.Error(err)
	})
	t.Run("Invalid tx hash", func(t *testing.T) {
		treeDepth := 10
		batchSize := 5
		params := BuildTestBatchUpdateTree(treeDepth, batchSize, nil, nil)

		circuit := createBatchUpdateCircuit(treeDepth, batchSize)
		witness := createBatchUpdateWitness(*params, 0, batchSize)

		// Swap two tx hashes to create an invalid order
		witness.TxHashes[0], witness.TxHashes[1] = witness.TxHashes[1], witness.TxHashes[0]

		err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
		assert.Error(err)
	})
}

func createBatchUpdateCircuit(treeDepth, batchSize int) BatchUpdateCircuit {
	circuit := BatchUpdateCircuit{
		PublicInputHash:     frontend.Variable(0),
		OldRoot:             frontend.Variable(0),
		NewRoot:             frontend.Variable(0),
		LeavesHashchainHash: frontend.Variable(0),
		TxHashes:            make([]frontend.Variable, batchSize),
		Leaves:              make([]frontend.Variable, batchSize),
		OldLeaves:           make([]frontend.Variable, batchSize),
		MerkleProofs:        make([][]frontend.Variable, batchSize),
		PathIndices:         make([]frontend.Variable, batchSize),
		Height:              uint32(treeDepth),
		BatchSize:           uint32(batchSize),
	}

	for i := range circuit.MerkleProofs {
		circuit.MerkleProofs[i] = make([]frontend.Variable, treeDepth)
	}

	return circuit
}

func createBatchUpdateWitness(params BatchUpdateParameters, startIndex, count int) BatchUpdateCircuit {
	witness := BatchUpdateCircuit{
		PublicInputHash:     frontend.Variable(params.PublicInputHash),
		OldRoot:             frontend.Variable(params.OldRoot),
		NewRoot:             frontend.Variable(params.NewRoot),
		LeavesHashchainHash: frontend.Variable(params.LeavesHashchainHash),
		TxHashes:            make([]frontend.Variable, count),
		Leaves:              make([]frontend.Variable, count),
		OldLeaves:           make([]frontend.Variable, count),
		MerkleProofs:        make([][]frontend.Variable, count),
		PathIndices:         make([]frontend.Variable, count),
		Height:              params.Height,
		BatchSize:           uint32(count),
	}

	for i := 0; i < count; i++ {
		witness.TxHashes[i] = frontend.Variable(params.TxHashes[i])
		witness.Leaves[i] = frontend.Variable(params.Leaves[i])
		witness.OldLeaves[i] = frontend.Variable(params.OldLeaves[i])
		witness.PathIndices[i] = frontend.Variable(params.PathIndices[i])
		witness.MerkleProofs[i] = make([]frontend.Variable, int(params.Height))
		for j := 0; j < int(params.Height); j++ {
			witness.MerkleProofs[i][j] = frontend.Variable(params.MerkleProofs[i][j])
		}
	}

	return witness
}
