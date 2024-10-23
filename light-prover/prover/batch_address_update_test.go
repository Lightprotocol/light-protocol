package prover

import (
	merkletree "light/light-prover/merkle-tree"
	"math/big"
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/test"
	iden3_poseidon "github.com/iden3/go-iden3-crypto/poseidon"
)

func TestBasicUpdate26(t *testing.T) {
	assert := test.NewAssert(t)

	params := BuildTestBatchAddressTreeAppend(26, 10, 0, nil, "")
	circuit := createAddressCircuit(params)
	witness := createAddressWitness(params)

	err := test.IsSolved(circuit, witness, ecc.BN254.ScalarField())
	assert.NoError(err)
}

// Tests
func TestBatchAddressTreeAppendCircuit(t *testing.T) {
	assert := test.NewAssert(t)

	t.Run("Functional tests", func(t *testing.T) {
		t.Run("Basic batch update - height 26", func(t *testing.T) {
			params := BuildTestBatchAddressTreeAppend(26, 10, 0, nil, "")
			circuit := createAddressCircuit(params)
			witness := createAddressWitness(params)

			err := test.IsSolved(circuit, witness, ecc.BN254.ScalarField())
			assert.NoError(err)
		})

		t.Run("Fill tree completely - height 10", func(t *testing.T) {
			treeHeight := uint32(10)
			batchSize := uint32(4)
			totalLeaves := uint32(1 << treeHeight)

			var params *BatchAddressTreeAppendParameters
			for startIndex := uint32(0); startIndex < totalLeaves; startIndex += batchSize {
				remainingLeaves := totalLeaves - startIndex
				if remainingLeaves < batchSize {
					batchSize = remainingLeaves
				}

				newParams := BuildTestBatchAddressTreeAppend(
					treeHeight,
					batchSize,
					startIndex,
					params,
					"",
				)

				circuit := createAddressCircuit(newParams)
				witness := createAddressWitness(newParams)

				err := test.IsSolved(circuit, witness, ecc.BN254.ScalarField())
				assert.NoError(err)

				params = newParams
			}
		})
	})

	t.Run("Failing cases", func(t *testing.T) {
		testCases := []struct {
			name        string
			invalidCase string
		}{
			{
				name:        "Invalid IndexedMerkleTree - wrong low element",
				invalidCase: "invalid_tree",
			},
			{
				name:        "Invalid IndexedMerkleTree - tree is full",
				invalidCase: "tree_full",
			},
			{
				name:        "Invalid new_element_value - outside range",
				invalidCase: "invalid_range",
			},
		}

		for _, tc := range testCases {
			t.Run(tc.name, func(t *testing.T) {
				params := BuildTestBatchAddressTreeAppend(26, 10, 0, nil, tc.invalidCase)
				circuit := createAddressCircuit(params)
				witness := createAddressWitness(params)

				err := test.IsSolved(circuit, witness, ecc.BN254.ScalarField())
				assert.Error(err)
			})
		}
	})
}

// Test helper functions
func BuildTestBatchAddressTreeAppend(
	treeHeight uint32,
	batchSize uint32,
	startIndex uint32,
	previousParams *BatchAddressTreeAppendParameters,
	invalidCase string,
) *BatchAddressTreeAppendParameters {
	var tree merkletree.PoseidonTree
	var oldSubTreeHashChain *big.Int
	var oldSubtrees []*big.Int

	// Initialize tree
	if previousParams == nil {
		tree = merkletree.NewTree(int(treeHeight))
		oldSubtrees = GetRightmostSubtrees(&tree, int(treeHeight))
		oldSubTreeHashChain = calculateHashChain(oldSubtrees, int(treeHeight))
	} else {
		tree = *previousParams.Tree.DeepCopy()
		oldSubtrees = GetRightmostSubtrees(&tree, int(treeHeight))
		oldSubTreeHashChain = previousParams.NewSubTreeHashChain
	}

	// Generate test data
	lowElementValues := make([]*big.Int, batchSize)
	lowElementNextValues := make([]*big.Int, batchSize)
	lowElementNextIndices := make([]uint32, batchSize)
	lowElementProofs := make([][]big.Int, batchSize)
	lowElementPathIndices := make([]uint32, batchSize)
	newElementValues := make([]*big.Int, batchSize)

	for i := uint32(0); i < batchSize; i++ {
		// Generate base values
		lowValue := new(big.Int).SetInt64(int64(i * 1000))
		nextValue := new(big.Int).SetInt64(int64((i + 1) * 1000))

		switch invalidCase {
		case "invalid_tree":
			lowValue.Add(lowValue, big.NewInt(999999))
		case "invalid_range":
			if i == 0 {
				nextValue = new(big.Int).Sub(lowValue, big.NewInt(1))
			}
		case "tree_full":
			startIndex = 1 << treeHeight
		default:
			// Valid case
			newElementValues[i] = new(big.Int).Add(lowValue, big.NewInt(500))
		}

		lowElementValues[i] = lowValue
		lowElementNextValues[i] = nextValue
		lowElementNextIndices[i] = i + 1
		lowElementPathIndices[i] = startIndex + i

		// Create and insert low element leaf
		lowLeaf, _ := iden3_poseidon.Hash([]*big.Int{
			lowValue,
			big.NewInt(int64(lowElementNextIndices[i])),
			nextValue,
		})

		// Get merkle proof for low element
		lowElementProofs[i] = tree.Update(int(lowElementPathIndices[i]), *lowLeaf)
	}

	// Calculate new root and hash chains
	newRoot := tree.Root.Value()
	newSubtrees := GetRightmostSubtrees(&tree, int(treeHeight))
	newSubTreeHashChain := calculateHashChain(newSubtrees, int(treeHeight))

	// Calculate hash chain for new leaves
	var newLeaves []*big.Int
	for i := uint32(0); i < batchSize; i++ {
		newLeaf, _ := iden3_poseidon.Hash([]*big.Int{
			newElementValues[i],
			big.NewInt(int64(lowElementNextIndices[i])),
			lowElementNextValues[i],
		})
		newLeaves = append(newLeaves, newLeaf)
	}
	hashchainHash := calculateHashChain(newLeaves, int(batchSize))

	// Calculate public input hash
	publicInputHash := calculateHashChain([]*big.Int{
		oldSubTreeHashChain,
		newSubTreeHashChain,
		&newRoot,
		hashchainHash,
		big.NewInt(int64(startIndex)),
	}, 5)

	return &BatchAddressTreeAppendParameters{
		PublicInputHash:       publicInputHash,
		OldSubTreeHashChain:   oldSubTreeHashChain,
		NewSubTreeHashChain:   newSubTreeHashChain,
		NewRoot:               &newRoot,
		HashchainHash:         hashchainHash,
		StartIndex:            startIndex,
		LowElementValues:      lowElementValues,
		LowElementNextValues:  lowElementNextValues,
		LowElementNextIndices: lowElementNextIndices,
		LowElementProofs:      lowElementProofs,
		LowElementPathIndices: lowElementPathIndices,
		NewElementValues:      newElementValues,
		Subtrees:              oldSubtrees,
		TreeHeight:            treeHeight,
		BatchSize:             batchSize,
		Tree:                  &tree,
	}
}

func createAddressCircuit(params *BatchAddressTreeAppendParameters) *BatchAddressTreeAppendCircuit {
	circuit := &BatchAddressTreeAppendCircuit{
		PublicInputHash:     frontend.Variable(0),
		OldSubTreeHashChain: frontend.Variable(0),
		NewSubTreeHashChain: frontend.Variable(0),
		NewRoot:             frontend.Variable(0),
		HashchainHash:       frontend.Variable(0),
		StartIndex:          frontend.Variable(0),

		LowElementValues:      make([]frontend.Variable, params.BatchSize),
		LowElementNextValues:  make([]frontend.Variable, params.BatchSize),
		LowElementNextIndices: make([]frontend.Variable, params.BatchSize),
		LowElementProofs:      make([][]frontend.Variable, params.BatchSize),
		LowElementPathIndices: make([]frontend.Variable, params.BatchSize),

		NewElementValues: make([]frontend.Variable, params.BatchSize),
		Subtrees:         make([]frontend.Variable, params.TreeHeight),

		BatchSize:  params.BatchSize,
		TreeHeight: params.TreeHeight,
	}

	// Initialize proofs array
	for i := range circuit.LowElementValues {
		circuit.LowElementProofs[i] = make([]frontend.Variable, params.TreeHeight)
		for j := range circuit.LowElementProofs[i] {
			circuit.LowElementProofs[i][j] = frontend.Variable(0)
		}
	}

	return circuit
}

func createAddressWitness(params *BatchAddressTreeAppendParameters) *BatchAddressTreeAppendCircuit {
	witness := createAddressCircuit(params)

	// Assign witness values
	witness.PublicInputHash = frontend.Variable(params.PublicInputHash)
	witness.OldSubTreeHashChain = frontend.Variable(params.OldSubTreeHashChain)
	witness.NewSubTreeHashChain = frontend.Variable(params.NewSubTreeHashChain)
	witness.NewRoot = frontend.Variable(params.NewRoot)
	witness.HashchainHash = frontend.Variable(params.HashchainHash)
	witness.StartIndex = frontend.Variable(params.StartIndex)

	for i := range witness.LowElementValues {
		witness.LowElementValues[i] = frontend.Variable(params.LowElementValues[i])
		witness.LowElementNextValues[i] = frontend.Variable(params.LowElementNextValues[i])
		witness.LowElementNextIndices[i] = frontend.Variable(params.LowElementNextIndices[i])
		witness.LowElementPathIndices[i] = frontend.Variable(params.LowElementPathIndices[i])
		witness.NewElementValues[i] = frontend.Variable(params.NewElementValues[i])

		for j := range params.LowElementProofs[i] {
			witness.LowElementProofs[i][j] = frontend.Variable(params.LowElementProofs[i][j])
		}
	}

	for i, subtree := range params.Subtrees {
		witness.Subtrees[i] = frontend.Variable(subtree)
	}

	return witness
}
