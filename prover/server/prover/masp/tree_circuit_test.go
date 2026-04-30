package masp

import (
	"math/big"
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/test"
)

func TestTreeCircuitMixed(t *testing.T) {
	w := mustWitness(t, SampleMixed)
	if w.StateProofs[0].Leaf.Cmp(w.StateLeaves[0]) != 0 {
		t.Fatal("state proof leaf should be the Light compressed-account hash")
	}
	if w.StateProofs[0].Leaf.Cmp(w.InCommits[0]) == 0 {
		t.Fatal("state proof leaf must not be the raw MASP UTXO commitment")
	}
	if err := test.IsSolved(NewTreeCircuit(w.N), w.TreeAssignment(), ecc.BN254.ScalarField()); err != nil {
		t.Fatalf("TreeCircuit IsSolved: %v", err)
	}
}

func TestTreeCircuitBinding(t *testing.T) {
	// If we build the tree_circuit witness against a different in_commit
	// than utxo_circuit hashed, both the Light leaf binding and nullifier
	// binding must fail.
	w := mustWitness(t, SampleMixed)
	assignment := w.TreeAssignment()
	assignment.InCommit[0] = big.NewInt(0xFEEDFACE)
	if err := test.IsSolved(NewTreeCircuit(w.N), assignment, ecc.BN254.ScalarField()); err == nil {
		t.Fatal("swapping in_commit should break the nullifier binding")
	}
}

func TestTreeCircuitLightAccountMetadataBinding(t *testing.T) {
	w := mustWitness(t, SampleMixed)

	for _, tc := range []struct {
		name   string
		tamper func(*TreeCircuit)
	}{
		{
			name: "owner",
			tamper: func(c *TreeCircuit) {
				c.AccountOwnerHash[0] = big.NewInt(0xA11CE)
			},
		},
		{
			name: "tree",
			tamper: func(c *TreeCircuit) {
				c.AccountTreeHash[0] = big.NewInt(0xB0B)
			},
		},
		{
			name: "discriminator",
			tamper: func(c *TreeCircuit) {
				c.AccountDiscriminator[0] = big.NewInt(0xCAFE)
			},
		},
	} {
		t.Run(tc.name, func(t *testing.T) {
			assignment := w.TreeAssignment()
			tc.tamper(assignment)
			if err := test.IsSolved(NewTreeCircuit(w.N), assignment, ecc.BN254.ScalarField()); err == nil {
				t.Fatalf("tampered Light account %s should fail binding", tc.name)
			}
		})
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
