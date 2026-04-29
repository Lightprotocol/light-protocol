package masp

import (
	"math/big"
	"testing"
)

func TestStateTreeSingleLeafInclusion(t *testing.T) {
	leafValue := big.NewInt(0xABCDEF)
	var leafIndex uint64 = 12345

	entries := map[uint64]*big.Int{leafIndex: leafValue}
	root, proofs := BuildSparseStateTree(entries)

	w, ok := proofs[leafIndex]
	if !ok {
		t.Fatal("missing proof for inserted leaf")
	}
	if w.Root.Cmp(root) != 0 {
		t.Fatalf("witness root mismatch")
	}

	computed := StatePathFold(w.Leaf, w.Siblings, w.Directions)
	if computed.Cmp(root) != 0 {
		t.Fatalf("inclusion verify failed")
	}

	// Reconstruct leaf index from directions.
	idx := LeafIndexFromDirections(w.Directions)
	if idx.Cmp(new(big.Int).SetUint64(leafIndex)) != 0 {
		t.Fatalf("leaf index reconstruction mismatch: got %s, want %d", idx, leafIndex)
	}

	// Tampering with a sibling breaks inclusion.
	bad := make([]*big.Int, len(w.Siblings))
	copy(bad, w.Siblings)
	bad[0] = new(big.Int).Add(bad[0], big.NewInt(1))
	if StatePathFold(w.Leaf, bad, w.Directions).Cmp(root) == 0 {
		t.Fatal("tampered sibling still verifies")
	}
}

func TestStateTreeMultipleLeavesDistinctRoots(t *testing.T) {
	root1, _ := BuildSparseStateTree(map[uint64]*big.Int{10: big.NewInt(1)})
	root2, _ := BuildSparseStateTree(map[uint64]*big.Int{10: big.NewInt(2)})
	if root1.Cmp(root2) == 0 {
		t.Fatal("different leaves produced same root")
	}

	// Two leaves in one tree — both inclusion proofs must verify against the same root.
	entries := map[uint64]*big.Int{
		5:  big.NewInt(100),
		99: big.NewInt(200),
	}
	root, proofs := BuildSparseStateTree(entries)
	for idx, w := range proofs {
		got := StatePathFold(w.Leaf, w.Siblings, w.Directions)
		if got.Cmp(root) != 0 {
			t.Fatalf("proof for idx=%d does not verify against root", idx)
		}
	}
}

func TestDirectionsRoundTrip(t *testing.T) {
	for _, idx := range []uint64{0, 1, 1024, (1 << 30) - 1, 1<<39 | 1} {
		dirs := DirectionsFromLeafIndex(idx)
		got := LeafIndexFromDirections(dirs)
		if got.Cmp(new(big.Int).SetUint64(idx)) != 0 {
			t.Fatalf("roundtrip mismatch for idx=%d: got=%s", idx, got)
		}
	}
}
