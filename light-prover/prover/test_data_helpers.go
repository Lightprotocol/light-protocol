package prover

import (
	"fmt"
	"github.com/consensys/gnark/frontend"
	merkletree "light/light-prover/merkle-tree"
	"math/big"
	"math/rand"

	"github.com/iden3/go-iden3-crypto/poseidon"
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
		oldSubtrees = tree.GetRightmostSubtrees(int(treeDepth))
		oldSubTreeHashChain = calculateHashChain(oldSubtrees, int(treeDepth))
	} else {
		tree = *previousParams.tree.DeepCopy()
		oldSubtrees = tree.GetRightmostSubtrees(int(treeDepth))
		oldSubTreeHashChain = previousParams.NewSubTreeHashChain
	}

	// Generate new leaves for this batch
	newLeaves := make([]*big.Int, batchSize)
	for i := uint32(0); i < batchSize; i++ {
		leaf, _ := poseidon.Hash([]*big.Int{big.NewInt(int64(startIndex + i))})
		newLeaves[i] = leaf
		tree.Update(int(startIndex)+int(i), *leaf)
	}

	newSubtrees := tree.GetRightmostSubtrees(int(treeDepth))
	newSubTreeHashChain := calculateHashChain(newSubtrees, int(treeDepth))
	newRoot := tree.Root.Value()
	hashchainHash := calculateHashChain(newLeaves, int(batchSize))

	publicInputHash := calculateHashChain([]*big.Int{
		oldSubTreeHashChain,
		newSubTreeHashChain,
		&newRoot,
		hashchainHash,
		big.NewInt(int64(startIndex))},
		5)
	params := BatchAppendParameters{
		PublicInputHash:     publicInputHash,
		OldSubTreeHashChain: oldSubTreeHashChain,
		NewSubTreeHashChain: newSubTreeHashChain,
		NewRoot:             &newRoot,
		HashchainHash:       hashchainHash,
		StartIndex:          startIndex,
		Leaves:              newLeaves,
		Subtrees:            oldSubtrees,
		TreeHeight:          treeDepth,
		tree:                &tree,
	}

	return params
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

	publicInputHash := calculateHashChain([]*big.Int{
		&oldRoot,
		&newRoot,
		leavesHashchainHash},
		3)
	return &BatchUpdateParameters{
		PublicInputHash:     publicInputHash,
		OldRoot:             &oldRoot,
		NewRoot:             &newRoot,
		LeavesHashchainHash: leavesHashchainHash,
		Leaves:              leaves,
		PathIndices:         pathIndices,
		MerkleProofs:        merkleProofs,
		Height:              uint32(treeDepth),
		BatchSize:           uint32(batchSize),
		Tree:                &tree,
	}
}

func hashChain(length int, inputs []frontend.Variable) *big.Int {
	if len(inputs) == 0 {
		return big.NewInt(0)
	}
	if len(inputs) == 1 {
		return inputs[0].(*big.Int)
	}

	hashChain := inputs[0].(*big.Int)
	for i := 1; i < length; i++ {
		hash, err := poseidon.Hash([]*big.Int{hashChain, inputs[i].(*big.Int)})
		if err != nil {
			panic(fmt.Sprintf("Failed to hash chain: %v", err))
		}
		hashChain = hash
	}
	return hashChain
}

type BatchAddressUpdateState struct {
	TreeRoot         big.Int
	LowElement       merkletree.IndexedElement
	NewElement       merkletree.IndexedElement
	LowElementProof  []big.Int
	NewElementProof  []big.Int
	CurrentTreeState *merkletree.IndexedMerkleTree
}

func BuildTestBatchAddressAppend(treeHeight uint32, batchSize uint32, startIndex uint32, previousParams *BatchAddressTreeAppendParameters, invalidCase string) *BatchAddressTreeAppendParameters {
	maxNodes := uint32(1 << treeHeight)

	// Early capacity checks
	if startIndex >= maxNodes {
		panic("Start index exceeds tree capacity")
	}

	// Calculate actual remaining capacity accounting for the need of two slots per operation
	remainingCapacity := maxNodes - startIndex
	if remainingCapacity < 1 { // Need at least 1 slot for proper linking
		panic("Insufficient tree capacity")
	}

	// Adjust batch size based on remaining capacity
	// We need one slot for each new element
	if batchSize > remainingCapacity {
		batchSize = remainingCapacity
	}

	// Further reduce batch size if we don't have enough slots
	if startIndex+batchSize > maxNodes {
		batchSize = maxNodes - startIndex
	}

	// Verify batch size is still valid
	if batchSize == 0 {
		panic("Batch size reduced to 0 due to capacity constraints")
	}

	// Minimum value gap to prevent small number issues
	minGap := new(big.Int).Exp(big.NewInt(2), big.NewInt(50), nil)

	var tree *merkletree.IndexedMerkleTree
	var oldRoot big.Int
	var lastBatchElement *merkletree.IndexedElement

	// Special handling for invalid cases
	switch invalidCase {
	case "invalid_tree":
		tree, _ := merkletree.NewIndexedMerkleTree(treeHeight)
		tree.Init()
		oldRoot := tree.Tree.Root.Value()
		newRoot := oldRoot

		params := BatchAddressTreeAppendParameters{
			PublicInputHash:  big.NewInt(0),
			OldRoot:          &oldRoot,
			NewRoot:          &newRoot,
			HashchainHash:    big.NewInt(0),
			StartIndex:       startIndex,
			OldLowElements:   make([]merkletree.IndexedElement, batchSize),
			LowElements:      make([]merkletree.IndexedElement, batchSize),
			NewElements:      make([]merkletree.IndexedElement, batchSize),
			LowElementProofs: make([][]big.Int, batchSize),
			NewElementProofs: make([][]big.Int, batchSize),
			TreeHeight:       treeHeight,
			BatchSize:        batchSize,
			Tree:             tree,
		}
		for i := uint32(0); i < batchSize; i++ {
			params.LowElementProofs[i] = make([]big.Int, treeHeight)
			params.NewElementProofs[i] = make([]big.Int, treeHeight)

			params.OldLowElements[i] = merkletree.IndexedElement{
				Value:     big.NewInt(int64(i)),
				NextValue: big.NewInt(int64(i + 1)),
				NextIndex: maxNodes + 1, // Invalid next index
				Index:     i,
			}

			params.LowElements[i] = params.OldLowElements[i]
			params.NewElements[i] = params.OldLowElements[i]
		}

		return &params

	case "tree_full":
		tree, _ := merkletree.NewIndexedMerkleTree(treeHeight)
		tree.Init()
		oldRoot := tree.Tree.Root.Value()
		newRoot := oldRoot

		params := BatchAddressTreeAppendParameters{
			PublicInputHash:  big.NewInt(0),
			OldRoot:          &oldRoot,
			NewRoot:          &newRoot,
			HashchainHash:    big.NewInt(0),
			StartIndex:       maxNodes - 1,
			OldLowElements:   make([]merkletree.IndexedElement, 1),
			LowElements:      make([]merkletree.IndexedElement, 1),
			NewElements:      make([]merkletree.IndexedElement, 1),
			LowElementProofs: make([][]big.Int, 1),
			NewElementProofs: make([][]big.Int, 1),
			TreeHeight:       treeHeight,
			BatchSize:        1,
			Tree:             tree,
		}

		params.LowElementProofs[0] = make([]big.Int, treeHeight)
		params.NewElementProofs[0] = make([]big.Int, treeHeight)

		return &params

	case "invalid_range":
		// Create tree with invalid range values
		tree, _ := merkletree.NewIndexedMerkleTree(treeHeight)
		tree.Init()
		oldRoot := tree.Tree.Root.Value()
		newRoot := oldRoot

		params := BatchAddressTreeAppendParameters{
			PublicInputHash:  big.NewInt(0),
			OldRoot:          &oldRoot,
			NewRoot:          &newRoot,
			HashchainHash:    big.NewInt(0),
			StartIndex:       maxNodes * 2,
			OldLowElements:   make([]merkletree.IndexedElement, batchSize),
			LowElements:      make([]merkletree.IndexedElement, batchSize),
			NewElements:      make([]merkletree.IndexedElement, batchSize),
			LowElementProofs: make([][]big.Int, batchSize),
			NewElementProofs: make([][]big.Int, batchSize),
			TreeHeight:       treeHeight,
			BatchSize:        batchSize,
			Tree:             tree,
		}

		for i := uint32(0); i < batchSize; i++ {
			params.LowElementProofs[i] = make([]big.Int, treeHeight)
			params.NewElementProofs[i] = make([]big.Int, treeHeight)

			params.OldLowElements[i] = merkletree.IndexedElement{
				Value:     big.NewInt(int64(maxNodes*2 + i)),
				NextValue: big.NewInt(int64(maxNodes*2 + i + 1)),
				NextIndex: i,
				Index:     i,
			}

			params.LowElements[i] = params.OldLowElements[i]
			params.NewElements[i] = params.OldLowElements[i]
		}

		return &params
	}

	if previousParams != nil {
		tree = previousParams.Tree.DeepCopy()
		oldRoot = *previousParams.NewRoot
		if len(previousParams.NewElements) > 0 {
			lastElement := previousParams.NewElements[previousParams.BatchSize-1]
			lastBatchElement = &lastElement
		}
	} else {
		var err error
		tree, err = merkletree.NewIndexedMerkleTree(treeHeight)
		if err != nil {
			panic(fmt.Sprintf("Failed to create indexed merkle tree: %v", err))
		}
		err = tree.Init()
		if err != nil {
			panic(fmt.Sprintf("Failed to initialize indexed merkle tree: %v", err))
		}
		oldRoot = tree.Tree.Root.Value()
	}

	updateStates := make([]BatchAddressUpdateState, batchSize)
	oldLowElements := make([]merkletree.IndexedElement, batchSize)
	lowElements := make([]merkletree.IndexedElement, batchSize)
	newElements := make([]merkletree.IndexedElement, batchSize)

	for i := uint32(0); i < batchSize; i++ {
		// Find low element
		var lowElement *merkletree.IndexedElement
		if i == 0 && lastBatchElement != nil {
			lowElement = lastBatchElement
		} else {
			lowElementIndex := tree.IndexArray.FindLowElementIndex(
				big.NewInt(int64(startIndex + i)),
			)
			lowElement = tree.IndexArray.Get(lowElementIndex)
			if lowElement == nil {
				batchSize = i
				break
			}
		}

		nextElementIndex := uint32(len(tree.IndexArray.Elements))
		if nextElementIndex >= maxNodes {
			batchSize = i
			break
		}

		oldLowElements[i] = *lowElement

		// Calculate available space
		diff := new(big.Int).Sub(lowElement.NextValue, lowElement.Value)

		// Ensure minimum gap
		if diff.Cmp(new(big.Int).Mul(minGap, big.NewInt(2))) <= 0 {
			batchSize = i
			break
		}

		// Calculate new value to maintain large gaps
		thirdsPoint := new(big.Int).Div(diff, big.NewInt(3))
		newValue := new(big.Int).Add(lowElement.Value, thirdsPoint)

		var nextValue *big.Int
		var nextIndex uint32

		if i == batchSize-1 && startIndex+batchSize < maxNodes {
			nextValue = lowElement.NextValue
			nextIndex = lowElement.NextIndex
		} else {
			nextValue = lowElement.NextValue
			nextIndex = nextElementIndex + 1
		}

		// Safety check for value ordering
		if newValue.Cmp(lowElement.Value) <= 0 || newValue.Cmp(nextValue) >= 0 {
			batchSize = i
			break
		}

		// Create elements with bounds checking
		if nextElementIndex < maxNodes {
			updatedLowElement := merkletree.IndexedElement{
				Value:     lowElement.Value,
				NextValue: newValue,
				NextIndex: nextElementIndex,
				Index:     lowElement.Index,
			}

			newElement := merkletree.IndexedElement{
				Value:     newValue,
				NextValue: nextValue,
				NextIndex: nextIndex,
				Index:     nextElementIndex,
			}

			lowElements[i] = updatedLowElement
			newElements[i] = newElement

			// Update tree state
			lowLeafHash, err := merkletree.HashIndexedElement(&updatedLowElement)
			if err != nil {
				panic(fmt.Sprintf("Failed to hash low leaf: %v", err))
			}

			lowProof := tree.Tree.GenerateProof(int(lowElement.Index))
			tree.Tree.Update(int(lowElement.Index), *lowLeafHash)

			newLeafHash, err := merkletree.HashIndexedElement(&newElement)
			if err != nil {
				panic(fmt.Sprintf("Failed to hash new leaf: %v", err))
			}

			newProof := tree.Tree.GenerateProof(int(nextElementIndex))
			tree.Tree.Update(int(nextElementIndex), *newLeafHash)

			updateStates[i] = BatchAddressUpdateState{
				TreeRoot:        tree.Tree.Root.Value(),
				LowElement:      updatedLowElement,
				NewElement:      newElement,
				LowElementProof: lowProof,
				NewElementProof: newProof,
			}

			tree.IndexArray.Elements[lowElement.Index] = updatedLowElement
			if int(nextElementIndex) >= len(tree.IndexArray.Elements) {
				tree.IndexArray.Elements = append(tree.IndexArray.Elements, newElement)
			} else {
				tree.IndexArray.Elements[nextElementIndex] = newElement
			}
			tree.IndexArray.CurrentNodeIndex = nextElementIndex
		} else {
			batchSize = i
			break
		}
	}

	// Adjust arrays to actual batch size
	if batchSize < uint32(len(oldLowElements)) {
		oldLowElements = oldLowElements[:batchSize]
		lowElements = lowElements[:batchSize]
		newElements = newElements[:batchSize]
		updateStates = updateStates[:batchSize]
	}

	newRoot := tree.Tree.Root.Value()
	var leafHashes []frontend.Variable
	for _, state := range updateStates {
		lowLeafHash, err := merkletree.HashIndexedElement(&state.LowElement)
		if err != nil {
			panic(err)
		}
		leafHashes = append(leafHashes, lowLeafHash)

		newLeafHash, err := merkletree.HashIndexedElement(&state.NewElement)
		if err != nil {
			panic(err)
		}
		leafHashes = append(leafHashes, newLeafHash)
	}

	leafHashChain := hashChain(len(leafHashes), leafHashes)

	publicInputHash := calculateHashChain([]*big.Int{
		&oldRoot,
		&newRoot,
		leafHashChain,
		big.NewInt(int64(startIndex)),
	}, 4)

	params := BatchAddressTreeAppendParameters{
		PublicInputHash:  publicInputHash,
		OldRoot:          &oldRoot,
		NewRoot:          &newRoot,
		HashchainHash:    leafHashChain,
		StartIndex:       startIndex,
		OldLowElements:   oldLowElements,
		LowElements:      lowElements,
		NewElements:      newElements,
		LowElementProofs: make([][]big.Int, batchSize),
		NewElementProofs: make([][]big.Int, batchSize),
		TreeHeight:       treeHeight,
		BatchSize:        batchSize,
		Tree:             tree,
	}

	for i := uint32(0); i < batchSize; i++ {
		params.LowElementProofs[i] = make([]big.Int, len(updateStates[i].LowElementProof))
		copy(params.LowElementProofs[i], updateStates[i].LowElementProof)

		params.NewElementProofs[i] = make([]big.Int, len(updateStates[i].NewElementProof))
		copy(params.NewElementProofs[i], updateStates[i].NewElementProof)
	}

	return &params
}
