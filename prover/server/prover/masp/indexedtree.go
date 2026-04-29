package masp

import (
	"fmt"
	"math/big"

	"light/light-prover/prover/masp/poseidon"
)

// IndexedTreeHeight matches StateTreeHeight (40) for the nullifier indexed
// Merkle tree. Light-protocol indexed trees follow the same height convention.
const IndexedTreeHeight = 40

// HighestAddressPlusOne is the sentinel next_value used by MASP's nullifier
// indexed tree. Set to `BN254_Fr_modulus - 1` so any valid BN254 Fr element
// is strictly less than it. This diverges from light-protocol's 248-bit
// address-space sentinel (2^248 - 297, see
// `program-libs/indexed-merkle-tree/src/lib.rs:45-46`) because MASP
// nullifiers are full BN254 Fr elements (up to ~2^254), not 248-bit
// addresses. See README for rationale.
var HighestAddressPlusOne, _ = new(big.Int).SetString(
	"30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000000",
	16,
)

// IndexedLeafHash computes the indexed-tree leaf digest:
//
//	leaf = Poseidon2(value, next_value)
//
// The leaf's `next_index` is NOT hashed here (it is stored on-chain but only
// used off-circuit for tree maintenance), matching
// `program-libs/indexed-array/src/array.rs:86-96`.
func IndexedLeafHash(value, nextValue *big.Int) *big.Int {
	h, err := poseidon.HashWithT(3, []*big.Int{value, nextValue})
	if err != nil {
		panic(err)
	}
	return h
}

// NonInclusionWitness is the witness tree_circuit consumes to prove that a
// given target value is NOT present in an indexed Merkle tree.
type NonInclusionWitness struct {
	Target     *big.Int // the value being proven absent
	LowValue   *big.Int // the greatest value in the tree that is < Target
	NextValue  *big.Int // that low element's next_value (first value in tree > Target, or HighestAddressPlusOne)
	Siblings   []*big.Int
	Directions []int
	Root       *big.Int
}

// VerifyNonInclusion reproduces the in-circuit check: fold the low element's
// leaf hash up the path, compare the root, and assert the strict range
// LowValue < Target < NextValue. Returns nil on success, an error otherwise.
// Useful as a sanity check for off-circuit witness construction.
func VerifyNonInclusion(w NonInclusionWitness) error {
	if w.LowValue.Cmp(w.Target) >= 0 {
		return fmt.Errorf("masp: non-inclusion requires LowValue < Target (got LowValue=%s, Target=%s)", w.LowValue, w.Target)
	}
	if w.Target.Cmp(w.NextValue) >= 0 {
		return fmt.Errorf("masp: non-inclusion requires Target < NextValue (got Target=%s, NextValue=%s)", w.Target, w.NextValue)
	}
	if len(w.Siblings) != IndexedTreeHeight || len(w.Directions) != IndexedTreeHeight {
		return fmt.Errorf("masp: indexed path length mismatch: siblings=%d directions=%d want=%d",
			len(w.Siblings), len(w.Directions), IndexedTreeHeight)
	}
	leafHash := IndexedLeafHash(w.LowValue, w.NextValue)
	computed := StatePathFold(leafHash, w.Siblings, w.Directions)
	if computed.Cmp(w.Root) != 0 {
		return fmt.Errorf("masp: indexed root mismatch: computed=%s want=%s", computed, w.Root)
	}
	return nil
}

// IndexedElement is an in-memory entry of the indexed linked list.
type IndexedElement struct {
	Index     uint64
	Value     *big.Int
	NextIndex uint64
}

// BuildIndexedTree constructs a height-40 indexed Merkle tree over a sorted
// list of values. The returned root, along with a non-inclusion witness
// builder, enables circuit tests. `values` must be unique and < HighestAddressPlusOne.
//
// The tree is initialized with a single sentinel element at index 0 with
// value 0 and NextIndex pointing to the first inserted real element (or 0
// again if no values are inserted). Each subsequent value is appended; the
// previous tail element's NextIndex is updated to point to the new element,
// and the new element's NextIndex is set to 0 only when it is the largest.
// For a correct indexed structure the caller is expected to pass values in
// ascending order. Duplicates panic.
//
// The implementation prioritizes clarity over scalability (O(n * height)
// rebuild for each insertion). Intended for small test fixtures only.
type IndexedTree struct {
	Elements    map[uint64]IndexedElement // keyed by Index
	LeafHashes  map[uint64]*big.Int       // keyed by Index
	Root        *big.Int
}

// NewIndexedTree creates an empty indexed tree seeded with the zero element
// at index 0 whose next_value is HighestAddressPlusOne (so the whole value
// range is vacant). This matches light-protocol's initialization where the
// first leaf at index 0 has value 0 and next_value = HIGHEST_ADDRESS_PLUS_ONE
// (see program-libs/indexed-merkle-tree/src/reference.rs).
func NewIndexedTree() *IndexedTree {
	t := &IndexedTree{
		Elements:   make(map[uint64]IndexedElement),
		LeafHashes: make(map[uint64]*big.Int),
	}
	t.Elements[0] = IndexedElement{
		Index:     0,
		Value:     new(big.Int),
		NextIndex: 0, // points to sentinel which has value = HighestAddressPlusOne (not stored as Element)
	}
	// The zero-element's leaf hash uses HighestAddressPlusOne as next_value.
	t.LeafHashes[0] = IndexedLeafHash(new(big.Int), HighestAddressPlusOne)
	t.rebuild()
	return t
}

// Insert appends a value to the indexed tree. Values must arrive in
// strictly ascending order. The previous tail element's next pointer is
// updated to this new element.
func (t *IndexedTree) Insert(value *big.Int) {
	if value.Sign() <= 0 || value.Cmp(HighestAddressPlusOne) >= 0 {
		panic(fmt.Sprintf("masp: indexed tree value out of range: %s", value))
	}
	// Find the element whose value is the current greatest (the tail).
	var tail IndexedElement
	first := true
	for _, e := range t.Elements {
		if first || e.Value.Cmp(tail.Value) > 0 {
			tail = e
			first = false
		}
	}
	if tail.Value.Cmp(value) >= 0 {
		panic(fmt.Sprintf("masp: indexed tree expects strictly ascending inserts: tail=%s new=%s", tail.Value, value))
	}
	newIdx := uint64(len(t.Elements))
	// Previous tail now points to new element.
	tail.NextIndex = newIdx
	t.Elements[tail.Index] = tail
	t.LeafHashes[tail.Index] = IndexedLeafHash(tail.Value, value)
	// New element is the new tail; its next_value is HighestAddressPlusOne.
	t.Elements[newIdx] = IndexedElement{Index: newIdx, Value: new(big.Int).Set(value), NextIndex: 0}
	t.LeafHashes[newIdx] = IndexedLeafHash(value, HighestAddressPlusOne)
	t.rebuild()
}

func (t *IndexedTree) rebuild() {
	entries := make(map[uint64]*big.Int, len(t.LeafHashes))
	for idx, h := range t.LeafHashes {
		entries[idx] = h
	}
	root, _ := BuildSparseStateTree(entries)
	t.Root = root
}

// NonInclusion returns a witness proving that `target` is not in the tree.
// Panics if the target is already present.
func (t *IndexedTree) NonInclusion(target *big.Int) NonInclusionWitness {
	if target.Sign() <= 0 || target.Cmp(HighestAddressPlusOne) >= 0 {
		panic(fmt.Sprintf("masp: NonInclusion target out of range: %s", target))
	}
	// Find low element: greatest value < target.
	var low IndexedElement
	found := false
	for _, e := range t.Elements {
		if e.Value.Cmp(target) >= 0 {
			continue
		}
		if !found || e.Value.Cmp(low.Value) > 0 {
			low = e
			found = true
		}
	}
	if !found {
		panic("masp: NonInclusion failed to find low element; tree corrupt")
	}
	// Next value is either another element's value or HighestAddressPlusOne.
	var nextValue *big.Int
	if low.NextIndex == 0 && low.Index == 0 && len(t.Elements) == 1 {
		// Empty tree: next_value is HighestAddressPlusOne.
		nextValue = new(big.Int).Set(HighestAddressPlusOne)
	} else if low.NextIndex == 0 && low.Index != 0 {
		// low is the tail element.
		nextValue = new(big.Int).Set(HighestAddressPlusOne)
	} else {
		next, ok := t.Elements[low.NextIndex]
		if !ok {
			panic("masp: NonInclusion next index missing")
		}
		nextValue = new(big.Int).Set(next.Value)
	}
	if nextValue.Cmp(target) <= 0 {
		panic(fmt.Sprintf("masp: NonInclusion: target=%s is already in (or past) the low element's range [%s, %s)",
			target, low.Value, nextValue))
	}
	// Build the path for the low element.
	entries := make(map[uint64]*big.Int, len(t.LeafHashes))
	for idx, h := range t.LeafHashes {
		entries[idx] = h
	}
	_, proofs := BuildSparseStateTree(entries)
	pw, ok := proofs[low.Index]
	if !ok {
		panic("masp: NonInclusion missing proof for low element")
	}
	return NonInclusionWitness{
		Target:     new(big.Int).Set(target),
		LowValue:   new(big.Int).Set(low.Value),
		NextValue:  nextValue,
		Siblings:   pw.Siblings,
		Directions: pw.Directions,
		Root:       new(big.Int).Set(t.Root),
	}
}
