package prover

import (
	"math/big"
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/test"
)

func TestBatchAppendWithSubtreesCircuit(t *testing.T) {
	assert := test.NewAssert(t)

	t.Run("Successful proofs", func(t *testing.T) {
		testCases := []struct {
			name       string
			treeDepth  uint32
			batchSize  uint32
			startIndex uint32
		}{
			{"Small batch", 26, 10, 0},
			{"Medium batch", 26, 100, 0},
			{"Large batch", 26, 1000, 0},
			{"Tree depth = 4", 4, 10, 0},
			{"Non-zero start index", 26, 100, 500},
			{"Start index near tree end", 10, 10, (1 << 10) - 15}, // 2^10 - 15 = 1009
		}

		for _, tc := range testCases {
			t.Run(tc.name, func(t *testing.T) {
				params := BuildAndUpdateBatchAppendWithSubtreesParameters(tc.treeDepth, tc.batchSize, tc.startIndex, nil)
				circuit := createCircuit(&params)
				witness := createWitness(&params)

				err := test.IsSolved(&circuit, witness, ecc.BN254.ScalarField())
				assert.NoError(err)
			})
		}
	})

	t.Run("Append 20 leaves in 2 proofs", func(t *testing.T) {
		treeDepth := uint32(26)
		batchSize := uint32(10)

		var params *BatchAppendWithSubtreesParameters
		for i := uint32(0); i < 2; i++ {
			startIndex := i * batchSize
			newParams := BuildAndUpdateBatchAppendWithSubtreesParameters(treeDepth, batchSize, startIndex, params)

			circuit := createCircuit(&newParams)
			witness := createWitness(&newParams)

			err := test.IsSolved(&circuit, witness, ecc.BN254.ScalarField())
			assert.NoError(err)

			params = &newParams
		}
	})

	t.Run("Append 50 leaves in 5 proofs", func(t *testing.T) {
		treeDepth := uint32(26)
		batchSize := uint32(10)

		var params *BatchAppendWithSubtreesParameters
		for i := uint32(0); i < 5; i++ {
			startIndex := i * batchSize
			newParams := BuildAndUpdateBatchAppendWithSubtreesParameters(treeDepth, batchSize, startIndex, params)

			circuit := createCircuit(&newParams)
			witness := createWitness(&newParams)

			err := test.IsSolved(&circuit, witness, ecc.BN254.ScalarField())
			assert.NoError(err)

			params = &newParams
		}
	})

	t.Run("Test fill tree completely", func(t *testing.T) {
		treeDepth := uint32(10)
		batchSize := uint32(10)
		totalLeaves := uint32(1 << treeDepth)

		var params *BatchAppendWithSubtreesParameters
		for startIndex := uint32(0); startIndex < totalLeaves; startIndex += batchSize {
			remainingLeaves := totalLeaves - startIndex
			if remainingLeaves < batchSize {
				batchSize = remainingLeaves
			}

			newParams := BuildAndUpdateBatchAppendWithSubtreesParameters(treeDepth, batchSize, startIndex, params)

			circuit := createCircuit(&newParams)
			witness := createWitness(&newParams)

			err := test.IsSolved(&circuit, witness, ecc.BN254.ScalarField())
			assert.NoError(err)

			params = &newParams
		}
	})

	t.Run("Multiple appends", func(t *testing.T) {
		treeDepth := uint32(26)
		batchSize := uint32(100)
		numAppends := 5

		var params *BatchAppendWithSubtreesParameters
		for i := 0; i < numAppends; i++ {
			startIndex := uint32(i * int(batchSize))
			newParams := BuildAndUpdateBatchAppendWithSubtreesParameters(treeDepth, batchSize, startIndex, params)

			circuit := createCircuit(&newParams)
			witness := createWitness(&newParams)

			err := test.IsSolved(&circuit, witness, ecc.BN254.ScalarField())
			assert.NoError(err)

			params = &newParams
		}
	})

	t.Run("Append at tree capacity", func(t *testing.T) {
		treeDepth := uint32(10) // Small depth for quicker testing
		batchSize := uint32(5)
		startIndex := uint32((1 << treeDepth) - batchSize) // 2^10 - 5 = 1019

		params := BuildAndUpdateBatchAppendWithSubtreesParameters(treeDepth, batchSize, startIndex, nil)
		circuit := createCircuit(&params)
		witness := createWitness(&params)

		err := test.IsSolved(&circuit, witness, ecc.BN254.ScalarField())
		assert.NoError(err)
	})

	t.Run("Failing cases", func(t *testing.T) {
		params := BuildAndUpdateBatchAppendWithSubtreesParameters(26, 100, 0, nil)

		testCases := []struct {
			name         string
			modifyParams func(*BatchAppendWithSubtreesParameters)
		}{
			{
				name: "Invalid OldSubTreeHashChain",
				modifyParams: func(p *BatchAppendWithSubtreesParameters) {
					p.OldSubTreeHashChain = big.NewInt(999)
				},
			},
			{
				name: "Invalid NewSubTreeHashChain",
				modifyParams: func(p *BatchAppendWithSubtreesParameters) {
					p.NewSubTreeHashChain = big.NewInt(999)
				},
			},
			{
				name: "Invalid NewRoot",
				modifyParams: func(p *BatchAppendWithSubtreesParameters) {
					p.NewRoot = big.NewInt(999)
				},
			},
			{
				name: "Invalid HashchainHash",
				modifyParams: func(p *BatchAppendWithSubtreesParameters) {
					p.HashchainHash = big.NewInt(999)
				},
			},
			{
				name: "Invalid Leaf",
				modifyParams: func(p *BatchAppendWithSubtreesParameters) {
					p.Leaves[0] = big.NewInt(999)
				},
			},
			{
				name: "Invalid Subtree",
				modifyParams: func(p *BatchAppendWithSubtreesParameters) {
					p.Subtrees[0] = big.NewInt(999)
				},
			},
			{
				name: "Mismatched BatchSize",
				modifyParams: func(p *BatchAppendWithSubtreesParameters) {
					p.Leaves = p.Leaves[:len(p.Leaves)-1] // Remove last leaf
				},
			},
			{
				name: "Start index exceeds tree capacity",
				modifyParams: func(p *BatchAppendWithSubtreesParameters) {
					p.StartIndex = 1 << p.TreeHeight // This should be invalid
				},
			},
		}

		for _, tc := range testCases {
			t.Run(tc.name, func(t *testing.T) {
				invalidParams := params
				tc.modifyParams(&invalidParams)

				circuit := createCircuit(&invalidParams)
				witness := createWitness(&invalidParams)

				err := test.IsSolved(&circuit, witness, ecc.BN254.ScalarField())
				assert.Error(err)
			})
		}

		t.Run("Invalid order of valid leaves", func(t *testing.T) {
			invalidParams := params
			invalidParams.Leaves[0], invalidParams.Leaves[1] = invalidParams.Leaves[1], invalidParams.Leaves[0]

			circuit := createCircuit(&invalidParams)
			witness := createWitness(&invalidParams)

			err := test.IsSolved(&circuit, witness, ecc.BN254.ScalarField())
			assert.Error(err)
		})

		t.Run("Invalid order of valid subtree hashes", func(t *testing.T) {
			invalidParams := params
			// Swap two subtree hashes to create an invalid order
			if len(invalidParams.Subtrees) >= 2 {
				invalidParams.Subtrees[0], invalidParams.Subtrees[1] = invalidParams.Subtrees[1], invalidParams.Subtrees[0]
			} else {
				t.Skip("Not enough subtrees to perform this test")
			}

			circuit := createCircuit(&invalidParams)
			witness := createWitness(&invalidParams)

			err := test.IsSolved(&circuit, witness, ecc.BN254.ScalarField())
			assert.Error(err, "Circuit should not be satisfied with invalid order of subtree hashes")
		})

		t.Run("Inconsistent subtree hashes", func(t *testing.T) {
			invalidParams := params
			// Change a subtree hash to an inconsistent value
			if len(invalidParams.Subtrees) > 0 {
				invalidParams.Subtrees[0] = new(big.Int).Add(invalidParams.Subtrees[0], big.NewInt(1))
			} else {
				t.Skip("No subtrees available to perform this test")
			}

			circuit := createCircuit(&invalidParams)
			witness := createWitness(&invalidParams)

			err := test.IsSolved(&circuit, witness, ecc.BN254.ScalarField())
			assert.Error(err, "Circuit should not be satisfied with inconsistent subtree hashes")
		})
	})
}

func createCircuit(params *BatchAppendWithSubtreesParameters) BatchAppendWithSubtreesCircuit {
	circuit := BatchAppendWithSubtreesCircuit{
		PublicInputHash:     frontend.Variable(0),
		OldSubTreeHashChain: frontend.Variable(0),
		NewSubTreeHashChain: frontend.Variable(0),
		NewRoot:             frontend.Variable(0),
		HashchainHash:       frontend.Variable(0),
		StartIndex:          frontend.Variable(0),
		Leaves:              make([]frontend.Variable, len(params.Leaves)),
		Subtrees:            make([]frontend.Variable, len(params.Subtrees)),
		BatchSize:           uint32(len(params.Leaves)),
		TreeHeight:          uint32(len(params.Subtrees)),
	}

	for i := range circuit.Leaves {
		circuit.Leaves[i] = frontend.Variable(0)
	}
	for i := range circuit.Subtrees {
		circuit.Subtrees[i] = frontend.Variable(0)
	}

	return circuit
}

func createWitness(params *BatchAppendWithSubtreesParameters) *BatchAppendWithSubtreesCircuit {
	witness := &BatchAppendWithSubtreesCircuit{
		PublicInputHash:     frontend.Variable(params.PublicInputHash),
		OldSubTreeHashChain: frontend.Variable(params.OldSubTreeHashChain),
		NewSubTreeHashChain: frontend.Variable(params.NewSubTreeHashChain),
		NewRoot:             frontend.Variable(params.NewRoot),
		HashchainHash:       frontend.Variable(params.HashchainHash),
		StartIndex:          frontend.Variable(params.StartIndex),
		Leaves:              make([]frontend.Variable, len(params.Leaves)),
		Subtrees:            make([]frontend.Variable, len(params.Subtrees)),
		BatchSize:           uint32(len(params.Leaves)),
		TreeHeight:          uint32(len(params.Subtrees)),
	}

	for i, leaf := range params.Leaves {
		witness.Leaves[i] = frontend.Variable(leaf)
	}
	for i, subtree := range params.Subtrees {
		witness.Subtrees[i] = frontend.Variable(subtree)
	}

	return witness
}

func BenchmarkBatchAppendWithSubtreesCircuit(b *testing.B) {
	params := BuildAndUpdateBatchAppendWithSubtreesParameters(26, 1000, 0, nil)
	circuit := createCircuit(&params)
	witness := createWitness(&params)

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = test.IsSolved(&circuit, witness, ecc.BN254.ScalarField())
	}
}
