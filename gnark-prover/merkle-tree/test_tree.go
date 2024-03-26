package merkle_tree

import (
	"fmt"
	"light/light-prover/prover"
	"math/big"
	"math/rand"
	"time"

	"github.com/iden3/go-iden3-crypto/poseidon"
)

type PoseidonNode interface {
	depth() int
	value() big.Int
	withValue(index int, val big.Int) PoseidonNode
	writeProof(index int, out []big.Int)
}

func indexIsLeft(index int, depth int) bool {
	return index&(1<<(depth-1)) == 0
}

func (node *PoseidonFullNode) depth() int {
	return node.dep
}

func (node *PoseidonEmptyNode) depth() int {
	return node.dep
}

func (node *PoseidonFullNode) value() big.Int {
	return node.val
}

func (node *PoseidonEmptyNode) value() big.Int {
	return node.emptyTreeValues[node.depth()]
}

func (node *PoseidonFullNode) withValue(index int, val big.Int) PoseidonNode {
	result := PoseidonFullNode{
		dep:   node.depth(),
		left:  node.left,
		right: node.right,
	}
	if node.depth() == 0 {
		result.val = val
	} else {
		if indexIsLeft(index, node.depth()) {
			result.left = node.left.withValue(index, val)
		} else {
			result.right = node.right.withValue(index, val)
		}
		result.initHash()
	}
	return &result
}

func (node *PoseidonEmptyNode) withValue(index int, val big.Int) PoseidonNode {
	result := PoseidonFullNode{
		dep: node.depth(),
	}
	if node.depth() == 0 {
		result.val = val
	} else {
		emptyChild := PoseidonEmptyNode{dep: node.depth() - 1, emptyTreeValues: node.emptyTreeValues}
		initializedChild := emptyChild.withValue(index, val)
		if indexIsLeft(index, node.depth()) {
			result.left = initializedChild
			result.right = &emptyChild
		} else {
			result.left = &emptyChild
			result.right = initializedChild
		}
		result.initHash()
	}
	return &result
}

func (node *PoseidonFullNode) writeProof(index int, out []big.Int) {
	if node.depth() == 0 {
		return
	}
	if indexIsLeft(index, node.depth()) {
		out[node.depth()-1] = node.right.value()
		node.left.writeProof(index, out)
	} else {
		out[node.depth()-1] = node.left.value()
		node.right.writeProof(index, out)
	}
}

func (node *PoseidonEmptyNode) writeProof(index int, out []big.Int) {
	for i := 0; i < node.depth(); i++ {
		out[i] = node.emptyTreeValues[i]
	}
}

type PoseidonFullNode struct {
	dep   int
	val   big.Int
	left  PoseidonNode
	right PoseidonNode
}

func (node *PoseidonFullNode) initHash() {
	leftVal := node.left.value()
	rightVal := node.right.value()
	newVal, _ := poseidon.Hash([]*big.Int{&leftVal, &rightVal})
	node.val = *newVal
}

type PoseidonEmptyNode struct {
	dep             int
	emptyTreeValues []big.Int
}

type PoseidonTree struct {
	root PoseidonNode
}

func (tree *PoseidonTree) Root() big.Int {
	return tree.root.value()
}

func (tree *PoseidonTree) Update(index int, value big.Int) []big.Int {
	tree.root = tree.root.withValue(index, value)
	proof := make([]big.Int, tree.root.depth())
	tree.root.writeProof(index, proof)
	return proof
}

func NewTree(depth int) PoseidonTree {
	initHashes := make([]big.Int, depth+1)
	for i := 1; i <= depth; i++ {
		val, _ := poseidon.Hash([]*big.Int{&initHashes[i-1], &initHashes[i-1]})
		initHashes[i] = *val
	}
	return PoseidonTree{root: &PoseidonEmptyNode{dep: depth, emptyTreeValues: initHashes}}
}

func BuildTestTree(depth int, numberOfUtxos int, random bool) prover.InclusionParameters {
	tree := NewTree(depth)

	rand.Seed(time.Now().UnixNano())
	var leaf *big.Int
	var inPathIndices int
	if random {
		leaf, _ = poseidon.Hash([]*big.Int{big.NewInt(rand.Int63())})
		inPathIndices = rand.Intn(depth)
	} else {
		leaf, _ = poseidon.Hash([]*big.Int{big.NewInt(1)})
		inPathIndices = 0
	}

	inPathElements := tree.Update(inPathIndices, *leaf)
	root := tree.Root()

	roots := make([]big.Int, numberOfUtxos)
	inPathIndicesBatch := make([]uint32, numberOfUtxos)
	inPathElementsBatch := make([][]big.Int, numberOfUtxos)
	leaves := make([]big.Int, numberOfUtxos)

	for i := 0; i < numberOfUtxos; i++ {
		roots[i] = root
		inPathIndicesBatch[i] = uint32(inPathIndices)
		inPathElementsBatch[i] = inPathElements
		leaves[i] = *leaf
	}

	return prover.InclusionParameters{
		Root:           roots,
		InPathIndices:  inPathIndicesBatch,
		InPathElements: inPathElementsBatch,
		Leaf:           leaves,
	}
}

func rangeIn(low, hi int) int {
	return low + rand.Intn(hi-low)
}

func BuildTestNonInclusionTree(depth int, numberOfUtxos int, random bool, valid bool) prover.NonInclusionParameters {
	tree := NewTree(depth)

	rand.Seed(time.Now().UnixNano())
	var value *big.Int = big.NewInt(0)
	var inPathIndices int
	var leafLower *big.Int
	var leafUpper *big.Int = big.NewInt(2)
	var leafIndex int
	if random {
		leafLower = big.NewInt(int64(rangeIn(0, 1000)))
		leafUpper.Add(leafUpper, leafLower)
		numberOfLeaves := 1 << depth
		leafIndex = rand.Intn(numberOfLeaves)
		if valid {
			value.Add(leafLower, big.NewInt(1))
		} else {
			var valueIsLower = rand.Intn(2)
			if valueIsLower == 1 {
				value.Add(leafLower, big.NewInt(-1))
			} else {
				value.Add(leafUpper, big.NewInt(1))
			}
		}
		inPathIndices = rand.Intn(depth)
	} else {
		leafLower = big.NewInt(1)
		leafUpper = big.NewInt(123)
		leafIndex = 1
		if valid {
			value = big.NewInt(2)
		} else {
			value = big.NewInt(4)
		}
		inPathIndices = 0
	}

	leafLowerRangeValue := make([]big.Int, numberOfUtxos)
	leafHigherRangeValue := make([]big.Int, numberOfUtxos)
	leafIndices := make([]uint32, numberOfUtxos)

	leaf, err := poseidon.Hash([]*big.Int{leafLower, big.NewInt(int64(leafIndex)), leafUpper})
	if err != nil {
		fmt.Println("error: ", err)
	}

	inPathElements := tree.Update(inPathIndices, *leaf)
	root := tree.Root()

	roots := make([]big.Int, numberOfUtxos)
	inPathIndicesBatch := make([]uint32, numberOfUtxos)
	inPathElementsBatch := make([][]big.Int, numberOfUtxos)
	values := make([]big.Int, numberOfUtxos)

	for i := 0; i < numberOfUtxos; i++ {
		roots[i] = root
		inPathIndicesBatch[i] = uint32(inPathIndices)
		inPathElementsBatch[i] = inPathElements
		values[i] = *value
		leafLowerRangeValue[i] = *leafLower
		leafHigherRangeValue[i] = *leafUpper
		leafIndices[i] = uint32(leafIndex)
	}

	return prover.NonInclusionParameters{
		Root:                 roots,
		InPathIndices:        inPathIndicesBatch,
		InPathElements:       inPathElementsBatch,
		Value:                values,
		LeafIndex:            leafIndices,
		LeafLowerRangeValue:  leafLowerRangeValue,
		LeafHigherRangeValue: leafHigherRangeValue,
	}
}
