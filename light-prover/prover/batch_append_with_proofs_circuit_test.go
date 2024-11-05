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
		startIndex := uint32(0)
		params := BuildTestBatchAppendWithProofsTree(treeDepth, batchSize, nil, &startIndex, false)

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
		startIndex := uint32(0)
		enable := true
		params := BuildTestBatchAppendWithProofsTree(treeDepth, batchSize, nil, &startIndex, enable)

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

	// 	t.Run("Fill up tree completely", func(t *testing.T) {
	// 		treeDepth := 8
	// 		batchSize := 4
	// 		totalLeaves := 1 << treeDepth
	// 		fmt.Println("totalLeaves", totalLeaves)
	// 		var tree = merkletree.NewTree(int(treeDepth))
	// 		for i := 0; i < totalLeaves/batchSize; i++ {
	// 			startIndex := uint32(i * batchSize)
	// 			params := BuildTestBatchAppendWithProofsTree(treeDepth, batchSize, &tree, &startIndex)

	// 			circuit := createBatchAppend2Circuit(treeDepth, batchSize)
	// 			witness := createBatchAppend2Witness(params, 0, batchSize)

	// 			err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
	// 			assert.NoError(err)
	// 			tree = *params.Tree.DeepCopy()
	// 		}
	// 	})

	// 	t.Run("Different tree depths and batch sizes", func(t *testing.T) {
	// 		testCases := []struct {
	// 			treeDepth int
	// 			batchSize int
	// 		}{
	// 			{4, 1},
	// 			{10, 100},
	// 			{26, 10},
	// 		}

	// 		for _, tc := range testCases {
	// 			t.Run(fmt.Sprintf("Depth:%d_Batch:%d", tc.treeDepth, tc.batchSize), func(t *testing.T) {
	// 				params := BuildTestBatchAppendWithProofsTree(tc.treeDepth, tc.batchSize, nil, nil)
	// 				circuit := createBatchAppend2Circuit(tc.treeDepth, tc.batchSize)
	// 				witness := createBatchAppend2Witness(params, 0, tc.batchSize)

	// 				err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
	// 				assert.NoError(err)
	// 			})
	// 		}
	// 	})

	// 	t.Run("Invalid NewRoot", func(t *testing.T) {
	// 		treeDepth := 10
	// 		batchSize := 5
	// 		params := BuildTestBatchAppendWithProofsTree(treeDepth, batchSize, nil, nil)

	// 		circuit := createBatchAppend2Circuit(treeDepth, batchSize)
	// 		witness := createBatchAppend2Witness(params, 0, batchSize)

	// 		// Modify NewRoot to make it invalid
	// 		witness.NewRoot = frontend.Variable(new(big.Int).Add(params.NewRoot, big.NewInt(1)))

	// 		err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
	// 		assert.Error(err)
	// 	})

	// 	t.Run("Invalid LeavesHashchainHash", func(t *testing.T) {
	// 		treeDepth := 10
	// 		batchSize := 5
	// 		params := BuildTestBatchAppendWithProofsTree(treeDepth, batchSize, nil, nil)

	// 		circuit := createBatchAppend2Circuit(treeDepth, batchSize)
	// 		witness := createBatchAppend2Witness(params, 0, batchSize)

	// 		// Modify LeavesHashchainHash to make it invalid
	// 		witness.LeavesHashchainHash = frontend.Variable(new(big.Int).Add(params.LeavesHashchainHash, big.NewInt(1)))

	// 		err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
	// 		assert.Error(err)
	// 	})

	// 	t.Run("Invalid leaf", func(t *testing.T) {
	// 		treeDepth := 10
	// 		batchSize := 5
	// 		params := BuildTestBatchAppendWithProofsTree(treeDepth, batchSize, nil, nil)

	// 		circuit := createBatchAppend2Circuit(treeDepth, batchSize)
	// 		witness := createBatchAppend2Witness(params, 0, batchSize)

	// 		// Modify one leaf to make it invalid
	// 		witness.Leaves[0] = frontend.Variable(new(big.Int).Add(params.Leaves[0], big.NewInt(1)))

	// 		err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
	// 		assert.Error(err)
	// 	})

	// 	t.Run("Invalid order of leaves", func(t *testing.T) {
	// 		treeDepth := 10
	// 		batchSize := 5
	// 		params := BuildTestBatchAppendWithProofsTree(treeDepth, batchSize, nil, nil)

	// 		circuit := createBatchAppend2Circuit(treeDepth, batchSize)
	// 		witness := createBatchAppend2Witness(params, 0, batchSize)

	// 		// Swap two leaves to create an invalid order
	// 		witness.Leaves[0], witness.Leaves[1] = witness.Leaves[1], witness.Leaves[0]

	// 		err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
	// 		assert.Error(err)
	// 	})
	// 	t.Run("Invalid tx hash", func(t *testing.T) {
	// 		treeDepth := 10
	// 		batchSize := 5
	// 		params := BuildTestBatchAppendWithProofsTree(treeDepth, batchSize, nil, nil)

	// 		circuit := createBatchAppend2Circuit(treeDepth, batchSize)
	// 		witness := createBatchAppend2Witness(params, 0, batchSize)

	// 		// Swap two tx hashes to create an invalid order
	// 		witness.OldLeaves[0], witness.OldLeaves[1] = witness.OldLeaves[1], witness.OldLeaves[0]

	//		err := test.IsSolved(&circuit, &witness, ecc.BN254.ScalarField())
	//		assert.Error(err)
	//	})
}

func createBatchAppendWithProofCircuit(treeDepth, batchSize int) BatchAppendWithProofsCircuit {
	circuit := BatchAppendWithProofsCircuit{
		PublicInputHash:     frontend.Variable(0),
		OldRoot:             frontend.Variable(0),
		NewRoot:             frontend.Variable(0),
		LeavesHashchainHash: frontend.Variable(0),
		OldLeaves:           make([]frontend.Variable, batchSize),
		Leaves:              make([]frontend.Variable, batchSize),
		MerkleProofs:        make([][]frontend.Variable, batchSize),
		StartIndex:          frontend.Variable(0),
		Height:              uint32(treeDepth),
		BatchSize:           uint32(batchSize),
	}

	for i := range circuit.MerkleProofs {
		circuit.MerkleProofs[i] = make([]frontend.Variable, treeDepth)
	}

	return circuit
}

func createBatchAppendWithProofWitness(params *BatchAppendWithProofsParameters, startIndex, count int) BatchAppendWithProofsCircuit {
	witness := BatchAppendWithProofsCircuit{
		PublicInputHash:     frontend.Variable(params.PublicInputHash),
		OldRoot:             frontend.Variable(params.OldRoot),
		NewRoot:             frontend.Variable(params.NewRoot),
		LeavesHashchainHash: frontend.Variable(params.LeavesHashchainHash),
		OldLeaves:           make([]frontend.Variable, count),
		Leaves:              make([]frontend.Variable, count),
		MerkleProofs:        make([][]frontend.Variable, count),
		StartIndex:          frontend.Variable(params.StartIndex),
		Height:              params.Height,
		BatchSize:           uint32(count),
	}

	for i := 0; i < count; i++ {
		witness.OldLeaves[i] = frontend.Variable(params.OldLeaves[i])
		witness.Leaves[i] = frontend.Variable(params.Leaves[i])
		witness.StartIndex = frontend.Variable(params.StartIndex)
		witness.MerkleProofs[i] = make([]frontend.Variable, int(params.Height))
		for j := 0; j < int(params.Height); j++ {
			witness.MerkleProofs[i][j] = frontend.Variable(params.MerkleProofs[i][j])
		}
	}

	return witness
}
