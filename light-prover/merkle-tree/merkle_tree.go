package merkle_tree

import (
	"github.com/iden3/go-iden3-crypto/poseidon"
	"math/big"
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

func BuildTestInsertionTree(depth int, batchSize int, random bool) prover.InsertionParameters {
	tree := NewTree(depth)

	var leaves []big.Int
	var merkleProofs [][]big.Int

	preRoot := tree.Root()

	for i := 0; i < batchSize; i++ {
		var leaf *big.Int
		var pathIndex int

		if random {
			leaf, _ = poseidon.Hash([]*big.Int{big.NewInt(rand.Int63())})
			pathIndex = rand.Intn(1 << depth)
		} else {
			leaf, _ = poseidon.Hash([]*big.Int{big.NewInt(int64(i + 1))})
			pathIndex = i
		}

		proof := tree.Update(pathIndex, *leaf)

		leaves = append(leaves, *leaf)
		merkleProofs = append(merkleProofs, proof)
	}

	postRoot := tree.Root()

	return prover.InsertionParameters{
		PreRoot:      preRoot,
		PostRoot:     postRoot,
		StartIndex:   0,
		Leaves:       leaves,
		MerkleProofs: merkleProofs,
	}
}

func BuildTestBatchUpdateTree(treeDepth int, batchSize int) *prover.BatchUpdateParameters {
	tree := NewTree(treeDepth)
	startIndex := uint32(rand.Intn(1 << treeDepth))

	oldLeaves := make([]big.Int, batchSize)
	newLeaves := make([]big.Int, batchSize)
	merkleProofs := make([][]big.Int, batchSize)

	fmt.Printf("Debug - BuildTestBatchUpdateTree: StartIndex: %d\n", startIndex)

	for i := 0; i < batchSize; i++ {
		index := int(startIndex) + i
		oldLeaf, _ := poseidon.Hash([]*big.Int{big.NewInt(int64(rand.Intn(1000000)))})
		newLeaf, _ := poseidon.Hash([]*big.Int{big.NewInt(int64(rand.Intn(1000000)))})

		oldLeaves[i] = *oldLeaf
		newLeaves[i] = *newLeaf

		merkleProofs[i] = tree.Update(index, *oldLeaf)

		root := tree.Root()
		fmt.Printf("Debug - BuildTestBatchUpdateTree: Round %d, Index: %d, OldLeaf: %s, NewLeaf: %s\n", i, index, oldLeaf.String(), newLeaf.String())
		fmt.Printf("Debug - BuildTestBatchUpdateTree: Intermediate Root: %s\n", root.String())
		fmt.Printf("Debug - BuildTestBatchUpdateTree: MerkleProof: %v\n", merkleProofs[i])

	}

	preRoot := tree.Root()
	fmt.Printf("Debug - BuildTestBatchUpdateTree: PreRoot: %s\n", preRoot.String())

	for i := 0; i < batchSize; i++ {
		index := int(startIndex) + i
		tree.Update(index, newLeaves[i])
		root := tree.Root()
		fmt.Printf("Debug - BuildTestBatchUpdateTree: After update %d, Root: %s\n", i, root.String())
	}

	postRoot := tree.Root()
	fmt.Printf("Debug - BuildTestBatchUpdateTree: PostRoot: %s\n", postRoot.String())

	return &prover.BatchUpdateParameters{
		PreRoot:      preRoot,
		PostRoot:     postRoot,
		StartIndex:   startIndex,
		OldLeaves:    oldLeaves,
		NewLeaves:    newLeaves,
		MerkleProofs: merkleProofs,
	}
}
