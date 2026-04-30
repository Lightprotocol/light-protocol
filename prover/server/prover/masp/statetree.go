package masp

import (
	"fmt"
	"math/big"

	"light/light-prover/prover/masp/poseidon"
)

// StateTreeHeight is the height of the binary state Merkle tree for Light
// compressed-account leaves. 40 levels → 2^40 leaves.
const StateTreeHeight = 40

// StateNodeHash computes the binary Merkle node hash = Poseidon2(left, right).
func StateNodeHash(left, right *big.Int) *big.Int {
	h, err := poseidon.HashWithT(3, []*big.Int{left, right})
	if err != nil {
		panic(err)
	}
	return h
}

// StatePathFold folds a leaf up a 40-level binary Merkle path.
//
// directions[j] tells the bit at level j: 0 means the current hash is the
// LEFT child at that level, 1 means RIGHT. The reconstructed leaf index is
// Σ directions[j] * 2^j (LSB at j=0, matching light-protocol's convention).
func StatePathFold(leaf *big.Int, siblings []*big.Int, directions []int) *big.Int {
	if len(siblings) != StateTreeHeight || len(directions) != StateTreeHeight {
		panic(fmt.Sprintf("masp: state path length mismatch: siblings=%d directions=%d want=%d",
			len(siblings), len(directions), StateTreeHeight))
	}
	h := new(big.Int).Set(leaf)
	for j := 0; j < StateTreeHeight; j++ {
		switch directions[j] {
		case 0:
			h = StateNodeHash(h, siblings[j])
		case 1:
			h = StateNodeHash(siblings[j], h)
		default:
			panic("masp: direction bit must be 0 or 1")
		}
	}
	return h
}

// LeafIndexFromDirections returns the integer formed by the 40 direction
// bits (LSB at j=0).
func LeafIndexFromDirections(directions []int) *big.Int {
	idx := new(big.Int)
	for j := len(directions) - 1; j >= 0; j-- {
		idx.Lsh(idx, 1)
		if directions[j] == 1 {
			idx.SetBit(idx, 0, 1)
		}
	}
	return idx
}

// DirectionsFromLeafIndex returns 40 direction bits LSB-first.
func DirectionsFromLeafIndex(index uint64) []int {
	out := make([]int, StateTreeHeight)
	for j := 0; j < StateTreeHeight; j++ {
		out[j] = int((index >> uint(j)) & 1)
	}
	return out
}

// stateEmptyCache returns the empty-node hash at each level from 0 (leaf) to
// height. empty[0] = 0; empty[k] = Poseidon2(empty[k-1], empty[k-1]).
func stateEmptyCache() []*big.Int {
	out := make([]*big.Int, StateTreeHeight+1)
	out[0] = new(big.Int)
	for k := 1; k <= StateTreeHeight; k++ {
		out[k] = StateNodeHash(out[k-1], out[k-1])
	}
	return out
}

// StateTreeWitness holds a leaf and its authentication path against a root.
type StateTreeWitness struct {
	Leaf       *big.Int
	Siblings   []*big.Int
	Directions []int
	Root       *big.Int
}

// BuildSparseStateTree inserts (leafIndex, leaf) pairs into a height-40
// sparse tree and returns the root plus a proof for each inserted leaf.
// Panics on duplicate indices.
func BuildSparseStateTree(entries map[uint64]*big.Int) (root *big.Int, proofs map[uint64]StateTreeWitness) {
	empty := stateEmptyCache()

	// nodes[level][index] — explicit only for populated paths.
	nodes := make([]map[uint64]*big.Int, StateTreeHeight+1)
	for i := range nodes {
		nodes[i] = make(map[uint64]*big.Int)
	}
	for idx, leaf := range entries {
		nodes[0][idx] = new(big.Int).Set(leaf)
	}

	// Build from level 0 up.
	for level := 0; level < StateTreeHeight; level++ {
		for idx := range nodes[level] {
			parentIdx := idx / 2
			if _, done := nodes[level+1][parentIdx]; done {
				continue
			}
			leftIdx := parentIdx * 2
			rightIdx := leftIdx + 1
			left, ok := nodes[level][leftIdx]
			if !ok {
				left = empty[level]
			}
			right, ok := nodes[level][rightIdx]
			if !ok {
				right = empty[level]
			}
			nodes[level+1][parentIdx] = StateNodeHash(left, right)
		}
	}

	root = nodes[StateTreeHeight][0]
	if root == nil {
		root = empty[StateTreeHeight]
	}

	proofs = make(map[uint64]StateTreeWitness, len(entries))
	for idx, leaf := range entries {
		siblings := make([]*big.Int, StateTreeHeight)
		directions := make([]int, StateTreeHeight)
		cur := idx
		for level := 0; level < StateTreeHeight; level++ {
			isRight := cur & 1
			directions[level] = int(isRight)
			sibIdx := cur ^ 1
			sib, ok := nodes[level][sibIdx]
			if !ok {
				sib = empty[level]
			}
			siblings[level] = sib
			cur >>= 1
		}
		proofs[idx] = StateTreeWitness{
			Leaf:       new(big.Int).Set(leaf),
			Siblings:   siblings,
			Directions: directions,
			Root:       new(big.Int).Set(root),
		}
	}
	return root, proofs
}
