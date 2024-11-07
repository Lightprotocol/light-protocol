package merkle_tree

import (
	"math/big"

	"github.com/iden3/go-iden3-crypto/poseidon"
)

type PoseidonNode interface {
	depth() int
	Value() big.Int
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

func (node *PoseidonFullNode) Value() big.Int {
	return node.val
}

func (node *PoseidonEmptyNode) Value() big.Int {
	return node.emptyTreeValues[node.depth()]
}

func (node *PoseidonFullNode) withValue(index int, val big.Int) PoseidonNode {
	result := PoseidonFullNode{
		dep:   node.depth(),
		Left:  node.Left,
		Right: node.Right,
	}
	if node.depth() == 0 {
		result.val = val
	} else {
		if indexIsLeft(index, node.depth()) {
			result.Left = node.Left.withValue(index, val)
		} else {
			result.Right = node.Right.withValue(index, val)
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
			result.Left = initializedChild
			result.Right = &emptyChild
		} else {
			result.Left = &emptyChild
			result.Right = initializedChild
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
		out[node.depth()-1] = node.Right.Value()
		node.Left.writeProof(index, out)
	} else {
		out[node.depth()-1] = node.Left.Value()
		node.Right.writeProof(index, out)
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
	Left  PoseidonNode
	Right PoseidonNode
}

func (node *PoseidonFullNode) initHash() {
	leftVal := node.Left.Value()
	rightVal := node.Right.Value()
	newVal, _ := poseidon.Hash([]*big.Int{&leftVal, &rightVal})
	node.val = *newVal
}

type PoseidonEmptyNode struct {
	dep             int
	emptyTreeValues []big.Int
}

type PoseidonTree struct {
	Root PoseidonNode
}

func (tree *PoseidonTree) Update(index int, value big.Int) []big.Int {
	tree.Root = tree.Root.withValue(index, value)
	proof := make([]big.Int, tree.Root.depth())
	tree.Root.writeProof(index, proof)
	return proof
}

func (tree *PoseidonTree) GetProofByIndex(index int) []big.Int {
	proof := make([]big.Int, tree.Root.depth())
	tree.Root.writeProof(index, proof)
	return proof
}

func NewTree(depth int) PoseidonTree {
	initHashes := make([]big.Int, depth+1)
	for i := 1; i <= depth; i++ {
		val, _ := poseidon.Hash([]*big.Int{&initHashes[i-1], &initHashes[i-1]})
		initHashes[i] = *val
	}
	return PoseidonTree{Root: &PoseidonEmptyNode{dep: depth, emptyTreeValues: initHashes}}
}

func (tree *PoseidonTree) DeepCopy() *PoseidonTree {
	if tree == nil {
		return nil
	}
	return &PoseidonTree{
		Root: deepCopyNode(tree.Root),
	}
}

func deepCopyNode(node PoseidonNode) PoseidonNode {
	if node == nil {
		return nil
	}

	switch n := node.(type) {
	case *PoseidonFullNode:
		return deepCopyFullNode(n)
	case *PoseidonEmptyNode:
		return deepCopyEmptyNode(n)
	default:
		panic("Unknown node type")
	}
}

func deepCopyFullNode(node *PoseidonFullNode) *PoseidonFullNode {
	if node == nil {
		return nil
	}
	return &PoseidonFullNode{
		dep:   node.dep,
		val:   *new(big.Int).Set(&node.val),
		Left:  deepCopyNode(node.Left),
		Right: deepCopyNode(node.Right),
	}
}

func deepCopyEmptyNode(node *PoseidonEmptyNode) *PoseidonEmptyNode {
	if node == nil {
		return nil
	}
	emptyTreeValues := make([]big.Int, len(node.emptyTreeValues))
	for i, v := range node.emptyTreeValues {
		emptyTreeValues[i] = *new(big.Int).Set(&v)
	}
	return &PoseidonEmptyNode{
		dep:             node.dep,
		emptyTreeValues: emptyTreeValues,
	}
}
