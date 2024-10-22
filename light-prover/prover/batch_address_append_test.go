package prover

import (
	"fmt"
	merkletree "light/light-prover/merkle-tree"
	"math/big"
	"testing"

	"github.com/stretchr/testify/assert"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/test"
)

func TestBatchAddressTreeAppendCircuit(t *testing.T) {
	assert := test.NewAssert(t)

	t.Run("Basic single batch operations", func(t *testing.T) {
		testCases := []struct {
			name       string
			treeHeight uint32
			batchSize  uint32
		}{
			{"Small tree single element", 4, 1},
			{"Small tree multiple elements", 4, 2},
			{"Medium tree batch", 10, 5},
			{"Large tree batch", 26, 10},
		}

		for _, tc := range testCases {
			t.Run(tc.name, func(t *testing.T) {
				params := BuildTestBatchAddressAppend(tc.treeHeight, tc.batchSize, 0, nil, "")
				circuit := createAddressCircuit(tc.treeHeight, tc.batchSize)
				witness := createAddressWitness(params)

				err := test.IsSolved(circuit, witness, ecc.BN254.ScalarField())
				assert.NoError(err)
			})
		}
	})

	t.Run("Small value gaps", func(t *testing.T) {
		treeHeight := uint32(8)
		batchSize := uint32(4)

		var previousParams *BatchAddressTreeAppendParameters

		// Process multiple batches and verify value gaps
		for i := uint32(0); i < 32; i++ {
			startIndex := i * batchSize
			params := BuildTestBatchAddressAppend(treeHeight, batchSize, startIndex, previousParams, "")

			// Verify minimum gaps between values
			for j := uint32(0); j < params.BatchSize; j++ {
				low := params.LowElements[j].Value
				mid := params.LowElements[j].NextValue
				high := params.NewElements[j].NextValue

				diff1 := new(big.Int).Sub(mid, low)
				diff2 := new(big.Int).Sub(high, mid)

				minGap := big.NewInt(1000)
				assert.True(diff1.Cmp(minGap) > 0, "Gap too small between low and mid values")
				assert.True(diff2.Cmp(minGap) > 0, "Gap too small between mid and high values")
			}

			previousParams = params
		}
	})

	t.Run("Near capacity handling", func(t *testing.T) {
		treeHeight := uint32(8)
		maxNodes := uint32(1 << treeHeight)
		startIndex := maxNodes - 8 // Test very close to capacity

		params := BuildTestBatchAddressAppend(treeHeight, 4, startIndex, nil, "")

		// Verify value gaps
		for i := uint32(0); i < params.BatchSize; i++ {
			gapLow := new(big.Int).Sub(params.LowElements[i].NextValue, params.LowElements[i].Value)
			gapHigh := new(big.Int).Sub(params.NewElements[i].NextValue, params.NewElements[i].Value)

			minGap := new(big.Int).Exp(big.NewInt(2), big.NewInt(50), nil)
			assert.True(gapLow.Cmp(minGap) > 0, "Low gap too small")
			assert.True(gapHigh.Cmp(minGap) > 0, "High gap too small")
		}
	})

	t.Run("Multiple sequential batches", func(t *testing.T) {
		treeHeight := uint32(10)
		batchSize := uint32(4)
		numBatches := uint32(3)

		var previousParams *BatchAddressTreeAppendParameters

		for i := uint32(0); i < numBatches; i++ {
			startIndex := i * batchSize
			t.Run(fmt.Sprintf("Batch %d", i), func(t *testing.T) {
				params := BuildTestBatchAddressAppend(
					treeHeight,
					batchSize,
					startIndex,
					previousParams,
					"",
				)

				circuit := createAddressCircuit(treeHeight, batchSize)
				witness := createAddressWitness(params)
				err := test.IsSolved(circuit, witness, ecc.BN254.ScalarField())
				assert.NoError(err)

				if previousParams != nil {
					assert.Equal(params.OldRoot, previousParams.NewRoot)
					assert.Equal(params.StartIndex, previousParams.StartIndex+previousParams.BatchSize)
					assert.NotNil(params.Tree)
				}

				previousParams = params
			})
		}
	})

	t.Run("Fill tree completely", func(t *testing.T) {
		treeHeight := uint32(8)
		batchSize := uint32(4)
		totalLeaves := uint32(1<<treeHeight) - 3

		var previousParams *BatchAddressTreeAppendParameters

		for startIndex := uint32(0); startIndex < totalLeaves; startIndex += batchSize {
			t.Run(fmt.Sprintf("Batch starting at %d", startIndex), func(t *testing.T) {
				remainingLeaves := totalLeaves - startIndex
				currentBatchSize := batchSize
				if remainingLeaves < batchSize {
					currentBatchSize = remainingLeaves
				}

				params := BuildTestBatchAddressAppend(
					treeHeight,
					currentBatchSize,
					startIndex,
					previousParams,
					"",
				)

				circuit := createAddressCircuit(treeHeight, batchSize)
				witness := createAddressWitness(params)

				err := test.IsSolved(circuit, witness, ecc.BN254.ScalarField())
				assert.NoError(err)

				previousParams = params
			})
		}
	})

	t.Run("Element linking verification", func(t *testing.T) {
		params := BuildTestBatchAddressAppend(10, 2, 0, nil, "")

		for i := uint32(0); i < params.BatchSize; i++ {
			t.Run(fmt.Sprintf("Element pair %d", i), func(t *testing.T) {
				if params.LowElements[i].NextValue.Cmp(params.NewElements[i].Value) != 0 {
					t.Errorf("Low element next value (%s) doesn't match new element value (%s)",
						params.LowElements[i].NextValue.String(),
						params.NewElements[i].Value.String())
				}

				if params.LowElements[i].NextIndex != params.NewElements[i].Index {
					t.Errorf("Low element next index (%d) doesn't match new element index (%d)",
						params.LowElements[i].NextIndex,
						params.NewElements[i].Index)
				}

				assert.Equal(len(params.LowElementProofs[i]), int(params.TreeHeight))
				assert.Equal(len(params.NewElementProofs[i]), int(params.TreeHeight))
			})
		}
	})

	t.Run("Hash chain verification", func(t *testing.T) {
		params := BuildTestBatchAddressAppend(10, 2, 0, nil, "")

		var leafHashesBI []*big.Int
		for i := uint32(0); i < params.BatchSize; i++ {
			lowLeafHash, err := merkletree.HashIndexedElement(&params.LowElements[i])
			assert.NoError(err)
			leafHashesBI = append(leafHashesBI, lowLeafHash)

			newLeafHash, err := merkletree.HashIndexedElement(&params.NewElements[i])
			assert.NoError(err)
			leafHashesBI = append(leafHashesBI, newLeafHash)
		}

		calculatedHashChainBI := calculateHashChain(leafHashesBI, len(leafHashesBI))
		assert.Equal(0, calculatedHashChainBI.Cmp(params.HashchainHash))
	})

	t.Run("Root verification", func(t *testing.T) {
		params := BuildTestBatchAddressAppend(10, 2, 0, nil, "")

		rootVal := params.Tree.Tree.Root.Value()
		rootCopy := new(big.Int).Set(&rootVal) // Create a copy of the root value

		assert.Equal(0, rootCopy.Cmp(params.NewRoot))
	})

	t.Run("Invalid cases", func(t *testing.T) {
		testCases := []struct {
			name        string
			invalidCase string
		}{
			{"Invalid low element", "invalid_tree"},
			{"Tree full", "tree_full"},
			{"Value out of range", "invalid_range"},
		}

		for _, tc := range testCases {
			t.Run(tc.name, func(t *testing.T) {
				params := BuildTestBatchAddressAppend(26, 10, 0, nil, tc.invalidCase)
				circuit := createAddressCircuit(26, 10)
				witness := createAddressWitness(params)

				err := test.IsSolved(circuit, witness, ecc.BN254.ScalarField())
				assert.Error(err)
			})
		}
	})
}

func TestCreateAddressCircuit(t *testing.T) {
	t.Run("Circuit creation", func(t *testing.T) {
		params := BuildTestBatchAddressAppend(10, 2, 0, nil, "")
		circuit := createAddressCircuit(10, 2)

		assert.Equal(t, params.BatchSize, circuit.BatchSize)
		assert.Equal(t, params.TreeHeight, circuit.TreeHeight)
		assert.Equal(t, int(params.BatchSize), len(circuit.LowElementProofs))
		assert.Equal(t, int(params.BatchSize), len(circuit.NewElementProofs))

		for i := uint32(0); i < params.BatchSize; i++ {
			assert.Equal(t, int(params.TreeHeight), len(circuit.LowElementProofs[i]))
			assert.Equal(t, int(params.TreeHeight), len(circuit.NewElementProofs[i]))
		}
	})
}

func TestHashChain(t *testing.T) {
	assert := test.NewAssert(t)

	t.Run("Empty chain", func(t *testing.T) {
		result := hashChain(0, []frontend.Variable{})
		assert.Equal(0, result.Cmp(big.NewInt(0)))
	})

	t.Run("Single element", func(t *testing.T) {
		input := big.NewInt(42)
		result := hashChain(1, []frontend.Variable{input})
		assert.Equal(0, result.Cmp(input))
	})

	t.Run("Multiple elements", func(t *testing.T) {
		inputs := []frontend.Variable{
			big.NewInt(1),
			big.NewInt(2),
			big.NewInt(3),
		}
		result := hashChain(len(inputs), inputs)
		assert.NotNil(result)
		assert.NotEqual(0, result.Cmp(big.NewInt(0)))
	})
}
