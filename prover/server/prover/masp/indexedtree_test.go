package masp

import (
	"math/big"
	"testing"
)

func TestIndexedTreeNonInclusionBetweenElements(t *testing.T) {
	tree := NewIndexedTree()
	tree.Insert(big.NewInt(10))
	tree.Insert(big.NewInt(20))
	tree.Insert(big.NewInt(30))

	// Value 25 lies between 20 and 30 → non-inclusion should verify.
	target := big.NewInt(25)
	w := tree.NonInclusion(target)
	if err := VerifyNonInclusion(w); err != nil {
		t.Fatalf("non-inclusion for 25 failed: %v", err)
	}

	// LowValue must be 20, NextValue must be 30.
	if w.LowValue.Cmp(big.NewInt(20)) != 0 {
		t.Fatalf("expected LowValue=20, got %s", w.LowValue)
	}
	if w.NextValue.Cmp(big.NewInt(30)) != 0 {
		t.Fatalf("expected NextValue=30, got %s", w.NextValue)
	}
}

func TestIndexedTreeNonInclusionEmptyTree(t *testing.T) {
	tree := NewIndexedTree()

	target := big.NewInt(42)
	w := tree.NonInclusion(target)
	if err := VerifyNonInclusion(w); err != nil {
		t.Fatalf("non-inclusion in empty tree failed: %v", err)
	}
	if w.LowValue.Sign() != 0 {
		t.Fatalf("empty tree low value should be 0, got %s", w.LowValue)
	}
	if w.NextValue.Cmp(HighestAddressPlusOne) != 0 {
		t.Fatalf("empty tree next value should be HighestAddressPlusOne, got %s", w.NextValue)
	}
}

func TestIndexedTreeNonInclusionPastTail(t *testing.T) {
	tree := NewIndexedTree()
	tree.Insert(big.NewInt(100))
	tree.Insert(big.NewInt(200))

	// Target > all elements → LowValue = 200, NextValue = HighestAddressPlusOne.
	target := big.NewInt(999)
	w := tree.NonInclusion(target)
	if err := VerifyNonInclusion(w); err != nil {
		t.Fatalf("non-inclusion past tail failed: %v", err)
	}
	if w.LowValue.Cmp(big.NewInt(200)) != 0 {
		t.Fatalf("expected LowValue=200, got %s", w.LowValue)
	}
	if w.NextValue.Cmp(HighestAddressPlusOne) != 0 {
		t.Fatalf("expected NextValue=HighestAddressPlusOne, got %s", w.NextValue)
	}
}

func TestIndexedTreeNonInclusionMustFailForInsertedValue(t *testing.T) {
	tree := NewIndexedTree()
	tree.Insert(big.NewInt(10))
	tree.Insert(big.NewInt(20))

	defer func() {
		if r := recover(); r == nil {
			t.Fatal("expected panic when asking for non-inclusion of an inserted value")
		}
	}()
	tree.NonInclusion(big.NewInt(20))
}
