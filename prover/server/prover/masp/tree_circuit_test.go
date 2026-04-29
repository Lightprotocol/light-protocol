package masp

import (
	"math/big"
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/test"
)

func TestTreeCircuitMixed(t *testing.T) {
	w := mustWitness(t, SampleMixed)
	if err := test.IsSolved(NewTreeCircuit(w.N), w.TreeAssignment(), ecc.BN254.ScalarField()); err != nil {
		t.Fatalf("TreeCircuit IsSolved: %v", err)
	}
}

func TestTreeCircuitBinding(t *testing.T) {
	// If we build the tree_circuit witness against a different in_commit
	// than utxo_circuit hashed, the nullifier binding (Poseidon3(in_commit,
	// leaf_index, dns) == public nullifier) must fail.
	w := mustWitness(t, SampleMixed)
	assignment := w.TreeAssignment()
	assignment.InCommit[0] = big.NewInt(0xFEEDFACE)
	if err := test.IsSolved(NewTreeCircuit(w.N), assignment, ecc.BN254.ScalarField()); err == nil {
		t.Fatal("swapping in_commit should break the nullifier binding")
	}
}

func TestTreeCircuitStateInclusionTamper(t *testing.T) {
	w := mustWitness(t, SampleMixed)
	assignment := w.TreeAssignment()
	// Tamper with the first sibling of input 0's state path.
	assignment.StatePath[0][0] = big.NewInt(0xDEADDEAD)
	if err := test.IsSolved(NewTreeCircuit(w.N), assignment, ecc.BN254.ScalarField()); err == nil {
		t.Fatal("tampered state sibling should fail inclusion")
	}
}
