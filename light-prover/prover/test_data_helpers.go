package prover

import (
	"fmt"
	"github.com/iden3/go-iden3-crypto/poseidon"
	merkletree "light/light-prover/merkle-tree"
	"math/big"
	"math/rand"
)

func rangeIn(low, hi int) int {
	return low + rand.Intn(hi-low)
}

func BuildTestTree(depth int, numberOfCompressedAccounts int, random bool) InclusionParameters {
	tree := merkletree.NewTree(depth)
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

func BuildValidTestNonInclusionTree(depth int, numberOfCompressedAccounts int, random bool) NonInclusionParameters {
	return BuildTestNonInclusionTree(depth, numberOfCompressedAccounts, random, true, false)
}

func BuildTestNonInclusionTree(depth int, numberOfCompressedAccounts int, random bool, valid bool, lowValue bool) NonInclusionParameters {
	tree := merkletree.NewTree(depth)

	var inputs = make([]NonInclusionInputs, numberOfCompressedAccounts)

	for i := 0; i < numberOfCompressedAccounts; i++ {
		var value = big.NewInt(0)
		var leafLower = big.NewInt(0)
		var leafUpper = big.NewInt(2)
		var pathIndex int
		var nextIndex int
		if random {
			leafLower = big.NewInt(int64(rangeIn(0, 1000)))
			leafUpper.Add(leafUpper, leafLower)
			numberOfLeaves := 1 << depth
			nextIndex = rand.Intn(numberOfLeaves)
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
			leafLower = big.NewInt(1)
			leafUpper = big.NewInt(123)
			nextIndex = 1
			if valid {
				value = big.NewInt(2)
			} else {
				value = big.NewInt(4)
			}
			pathIndex = 0
		}

		leaf, err := poseidon.Hash([]*big.Int{leafLower, big.NewInt(int64(nextIndex)), leafUpper})
		if err != nil {
			fmt.Println("error: ", err)
		}

		inputs[i].Value = *value
		inputs[i].PathIndex = uint32(pathIndex)
		inputs[i].PathElements = tree.Update(pathIndex, *leaf)
		inputs[i].Root = tree.Root.Value()
		inputs[i].LeafLowerRangeValue = *leafLower
		inputs[i].LeafHigherRangeValue = *leafUpper
		inputs[i].NextIndex = uint32(nextIndex)
	}

	return NonInclusionParameters{
		Inputs: inputs,
	}
}

func BuildAndUpdateBatchAppendParameters(treeDepth uint32, batchSize uint32, startIndex uint32, previousParams *BatchAppendParameters) BatchAppendParameters {
	var tree merkletree.PoseidonTree
	var oldSubTreeHashChain *big.Int
	var oldSubtrees []*big.Int

	if previousParams == nil {
		tree = merkletree.NewTree(int(treeDepth))
		// Generate and insert initial leaves
		for i := uint32(0); i < startIndex; i++ {
			leaf, _ := poseidon.Hash([]*big.Int{big.NewInt(int64(i))})
			tree.Update(int(i), *leaf)
		}
		oldSubtrees = GetRightmostSubtrees(&tree, int(treeDepth))
		oldSubTreeHashChain = calculateHashChain(oldSubtrees, int(treeDepth))
	} else {
		tree = *previousParams.tree.DeepCopy()
		oldSubtrees = GetRightmostSubtrees(&tree, int(treeDepth))
		oldSubTreeHashChain = previousParams.NewSubTreeHashChain
	}

	// Generate new leaves for this batch
	newLeaves := make([]*big.Int, batchSize)
	for i := uint32(0); i < batchSize; i++ {
		leaf, _ := poseidon.Hash([]*big.Int{big.NewInt(int64(startIndex + i))})
		newLeaves[i] = leaf
		tree.Update(int(startIndex)+int(i), *leaf)
	}

	newSubtrees := GetRightmostSubtrees(&tree, int(treeDepth))
	newSubTreeHashChain := calculateHashChain(newSubtrees, int(treeDepth))
	newRoot := tree.Root.Value()
	hashchainHash := calculateHashChain(newLeaves, int(batchSize))

	params := BatchAppendParameters{
		OldSubTreeHashChain: oldSubTreeHashChain,
		NewSubTreeHashChain: newSubTreeHashChain,
		NewRoot:             &newRoot,
		HashchainHash:       hashchainHash,
		StartIndex:          startIndex,
		Leaves:              newLeaves,
		Subtrees:            oldSubtrees,
		HashChainStartIndex: uint32(0),
		TreeHeight:          treeDepth,
		tree:                &tree,
	}

	return params
}

func GetRightmostSubtrees(tree *merkletree.PoseidonTree, depth int) []*big.Int {
	subtrees := make([]*big.Int, depth)
	for i := 0; i < depth; i++ {
		subtrees[i] = new(big.Int).SetBytes(ZERO_BYTES[i][:])
	}

	/*
		start at top x
		take left child hash as subtree
		if right node is not zero value, go down right
		if right node is zero value go down left node
	*/

	if fullNode, ok := tree.Root.(*merkletree.PoseidonFullNode); ok {
		current := fullNode
		level := depth - 1
		for current != nil && level >= 0 {
			if fullLeft, ok := current.Left.(*merkletree.PoseidonFullNode); ok {
				value := fullLeft.Value()
				subtrees[level] = &value
				if fullRight, ok := current.Right.(*merkletree.PoseidonFullNode); ok {
					current = fullRight
				} else {
					current = fullLeft
				}
			}
			level--
		}
	}
	return subtrees
}

func calculateHashChain(hashes []*big.Int, length int) *big.Int {
	if len(hashes) == 0 {
		return big.NewInt(0)
	}
	if len(hashes) == 1 {
		return hashes[0]
	}

	hashChain := hashes[0]
	for i := 1; i < length; i++ {
		hashChain, _ = poseidon.Hash([]*big.Int{hashChain, hashes[i]})
	}
	return hashChain
}

func BuildTestBatchUpdateTree(treeDepth int, batchSize int, previousTree *merkletree.PoseidonTree, startIndex *uint32) *BatchUpdateParameters {
	var tree merkletree.PoseidonTree

	if previousTree == nil {
		tree = merkletree.NewTree(treeDepth)
	} else {
		tree = *previousTree.DeepCopy()
	}

	leaves := make([]*big.Int, batchSize)
	merkleProofs := make([][]big.Int, batchSize)
	pathIndices := make([]uint32, batchSize)
	emptyLeaf := big.NewInt(0)

	usedIndices := make(map[uint32]bool)

	for i := 0; i < batchSize; i++ {
		leaf, _ := poseidon.Hash([]*big.Int{big.NewInt(int64(rand.Intn(1000000)))})
		leaves[i] = leaf

		if startIndex != nil {
			// Sequential filling
			pathIndices[i] = *startIndex + uint32(i)
		} else {
			// Random filling with uniqueness check
			for {
				index := uint32(rand.Intn(1 << uint(treeDepth)))
				if !usedIndices[index] {
					pathIndices[i] = index
					usedIndices[index] = true
					break
				}
			}
		}

		tree.Update(int(pathIndices[i]), *leaf)
	}

	oldRoot := tree.Root.Value()

	for i := 0; i < batchSize; i++ {
		merkleProofs[i] = tree.Update(int(pathIndices[i]), *leaves[i])
		tree.Update(int(pathIndices[i]), *emptyLeaf)
	}

	leavesHashchainHash := calculateHashChain(leaves, batchSize)
	newRoot := tree.Root.Value()

	return &BatchUpdateParameters{
		OldRoot:             &oldRoot,
		NewRoot:             &newRoot,
		LeavesHashchainHash: leavesHashchainHash,
		Leaves:              leaves,
		PathIndices:         pathIndices,
		MerkleProofs:        merkleProofs,
		HashChainStartIndex: 0,
		Height:              uint32(treeDepth),
		BatchSize:           uint32(batchSize),
		Tree:                &tree,
	}
}
