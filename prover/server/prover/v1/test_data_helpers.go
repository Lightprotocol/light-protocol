package v1

import (
	merkle_tree "light/light-prover/merkle-tree"
	"math/big"
	"math/rand"

	"github.com/iden3/go-iden3-crypto/poseidon"
)

func rangeIn(low, hi int) int {
	return low + rand.Intn(hi-low)
}

// BuildTestTree creates test inclusion parameters
func BuildTestTree(depth int, numberOfCompressedAccounts int, random bool) InclusionParameters {
	tree := merkle_tree.NewTree(depth)
	var leaf *big.Int
	var pathIndex int
	if random {
		leaf, _ = poseidon.Hash([]*big.Int{big.NewInt(rand.Int63())})
		pathIndex = rand.Intn(depth)
	} else {
		leaf, _ = poseidon.Hash([]*big.Int{big.NewInt(1)})
		pathIndex = 0
	}

	var inputs = make([]InclusionInputs, numberOfCompressedAccounts)

	for i := 0; i < numberOfCompressedAccounts; i++ {
		inputs[i].Leaf = *leaf
		inputs[i].PathIndex = uint32(pathIndex)
		inputs[i].PathElements = tree.Update(pathIndex, *leaf)
		inputs[i].Root = tree.Root.Value()
	}

	return InclusionParameters{
		Inputs: inputs,
	}
}

// BuildValidTestNonInclusionTree creates valid non-inclusion test parameters
func BuildValidTestNonInclusionTree(depth int, numberOfCompressedAccounts int, random bool) NonInclusionParameters {
	return BuildTestNonInclusionTree(depth, numberOfCompressedAccounts, random, true, false)
}

// BuildTestNonInclusionTree creates non-inclusion test parameters with configurable validity
func BuildTestNonInclusionTree(depth int, numberOfCompressedAccounts int, random bool, valid bool, lowValue bool) NonInclusionParameters {
	tree := merkle_tree.NewTree(depth)

	var inputs = make([]NonInclusionInputs, numberOfCompressedAccounts)

	for i := 0; i < numberOfCompressedAccounts; i++ {
		var value = big.NewInt(0)
		var leafLower = big.NewInt(0)
		var leafUpper = big.NewInt(2)
		var pathIndex int

		if random {
			leafLower = big.NewInt(int64(rangeIn(0, 1000)))
			leafUpper.Add(leafUpper, leafLower)
			if valid {
				value.Add(leafLower, big.NewInt(1))
			} else {
				if lowValue {
					value.Sub(leafLower, big.NewInt(1))
				} else {
					value.Add(leafUpper, big.NewInt(1))
				}
			}
			pathIndex = rand.Intn(depth)
		} else {
			leafLower = big.NewInt(10)
			leafUpper = big.NewInt(123)
			if valid {
				value = big.NewInt(50) // Valid value between 10 and 123
			} else {
				if lowValue {
					value = big.NewInt(5) // Invalid value less than 10
				} else {
					value = big.NewInt(200) // Invalid value greater than 123
				}
			}
			pathIndex = 0
		}

		// Hash the leaf for indexed merkle tree
		// V1 circuits use 3-input hash with NextIndex in the middle
		nextIndex := big.NewInt(0) // For test purposes, using 0 as NextIndex
		hash, _ := poseidon.Hash([]*big.Int{leafLower, nextIndex, leafUpper})
		pathElements := tree.Update(pathIndex, *hash)
		root := tree.Root.Value()

		inputs[i].Root = root
		inputs[i].Value = *value
		inputs[i].LeafLowerRangeValue = *leafLower
		inputs[i].LeafHigherRangeValue = *leafUpper
		inputs[i].NextIndex = uint32(0) // Set NextIndex explicitly
		inputs[i].PathIndex = uint32(pathIndex)
		inputs[i].PathElements = pathElements
	}

	return NonInclusionParameters{
		Inputs: inputs,
	}
}

// BuildIndexedMerkleTree creates an indexed merkle tree for testing
func BuildIndexedMerkleTree(depth int) *IndexedMerkleTree {
	tree := merkle_tree.NewTree(depth)
	return &IndexedMerkleTree{
		tree: &tree,
	}
}

// IndexedMerkleTree wrapper for testing
type IndexedMerkleTree struct {
	tree *merkle_tree.PoseidonTree
}

// GenerateProof generates a merkle proof for the given index
func (t *IndexedMerkleTree) GenerateProof(index int) ([]big.Int, int) {
	// Initialize with the max field value scenario
	leafLower := big.NewInt(0)
	leafUpper := new(big.Int)
	leafUpper.SetString("00ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff", 16)
	nextIndex := big.NewInt(0) // V1 circuits use NextIndex in the middle

	// Create the leaf hash - V1 circuits use 3-input hash
	hash, _ := poseidon.Hash([]*big.Int{leafLower, nextIndex, leafUpper})

	// Update tree at index and get path
	pathElements := t.tree.Update(index, *hash)

	return pathElements, index
}

// Root returns the current root of the tree
func (t *IndexedMerkleTree) Root() *big.Int {
	root := t.tree.Root.Value()
	return &root
}

// BuildValidCombinedParameters creates valid combined circuit test parameters
func BuildValidCombinedParameters(inclusionDepth, nonInclusionDepth int,
	inclusionAccounts, nonInclusionAccounts int) CombinedParameters {

	inclusionParams := BuildTestTree(inclusionDepth, inclusionAccounts, false)
	nonInclusionParams := BuildValidTestNonInclusionTree(nonInclusionDepth, nonInclusionAccounts, false)

	return CombinedParameters{
		InclusionParameters:    inclusionParams,
		NonInclusionParameters: nonInclusionParams,
	}
}