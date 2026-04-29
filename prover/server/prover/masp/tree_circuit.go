package masp

import (
	"fmt"
	"math/big"

	"github.com/consensys/gnark/frontend"
)

// TreeCircuit proves:
//   - Per-input in_commit_i is included in state_roots[i] via a 40-level
//     binary Merkle path.
//   - Per-input nullifier (public) is NOT in nullifier_roots[i] via an
//     indexed-tree non-inclusion proof (low leaf inclusion + strict range).
//   - nullifiers[i] == Poseidon3(in_commit_i, leaf_index_i, dns_i) where
//     leaf_index_i is reconstructed from the Merkle path directions and
//     dns_i is a per-input witness (the domain-nullifier-secret).
//
// The shared public `nullifiers[i]` binds this proof to utxo_circuit: any
// mismatch in (in_commit, leaf_index) between the two circuits makes the
// nullifier equality fail in at least one of them.
type TreeCircuit struct {
	NInputs int `gnark:"-"`

	// Per-input private witness.
	InCommit   []frontend.Variable
	StatePath  [][]frontend.Variable // [N][height] siblings
	StateDirs  [][]frontend.Variable // [N][height] direction bits (0 = current is left, 1 = right)
	DomainDNS  []frontend.Variable   // per-input domain_nullifier_secret

	// Non-inclusion witnesses for nullifiers.
	NfLowValue   []frontend.Variable
	NfNextValue  []frontend.Variable
	NfLowPath    [][]frontend.Variable // [N][height] siblings
	NfLowDirs    [][]frontend.Variable // [N][height] direction bits

	// ==== Logical "public" inputs, folded into PublicInputsHash ====
	StateRoots     []frontend.Variable
	NullifierRoots []frontend.Variable
	Nullifiers     []frontend.Variable

	// PublicInputsHash is the one real public input. It folds three
	// sub-chains (each of length N), in this order:
	//
	//   StateRootsChain     = HashChain(StateRoots[0..N-1])
	//   NullifierRootsChain = HashChain(NullifierRoots[0..N-1])
	//   NullifierChain      = HashChain(Nullifiers[0..N-1])
	//   PublicInputsHash    = HashChain([StateRootsChain, NullifierRootsChain, NullifierChain])
	//
	// NullifierChain matches utxo_circuit's binding term — both circuits
	// compute it from their nullifier witnesses and the on-chain verifier
	// feeds the same digest into both Groth16 verifies.
	PublicInputsHash frontend.Variable `gnark:",public"`
}

// NewTreeCircuit returns a compile-ready circuit skeleton for N inputs.
// Height is fixed to StateTreeHeight (== IndexedTreeHeight == 40).
func NewTreeCircuit(n int) *TreeCircuit {
	if n < 1 {
		panic(fmt.Sprintf("masp.NewTreeCircuit: n must be >= 1, got %d", n))
	}
	c := &TreeCircuit{NInputs: n}
	c.InCommit = make([]frontend.Variable, n)
	c.DomainDNS = make([]frontend.Variable, n)
	c.StatePath = make([][]frontend.Variable, n)
	c.StateDirs = make([][]frontend.Variable, n)
	c.NfLowValue = make([]frontend.Variable, n)
	c.NfNextValue = make([]frontend.Variable, n)
	c.NfLowPath = make([][]frontend.Variable, n)
	c.NfLowDirs = make([][]frontend.Variable, n)
	for i := 0; i < n; i++ {
		c.StatePath[i] = make([]frontend.Variable, StateTreeHeight)
		c.StateDirs[i] = make([]frontend.Variable, StateTreeHeight)
		c.NfLowPath[i] = make([]frontend.Variable, IndexedTreeHeight)
		c.NfLowDirs[i] = make([]frontend.Variable, IndexedTreeHeight)
	}
	c.StateRoots = make([]frontend.Variable, n)
	c.NullifierRoots = make([]frontend.Variable, n)
	c.Nullifiers = make([]frontend.Variable, n)
	return c
}

func (c *TreeCircuit) Define(api frontend.API) error {
	if c.NInputs < 1 {
		return fmt.Errorf("masp.TreeCircuit: NInputs must be set to a positive value")
	}

	for i := 0; i < c.NInputs; i++ {
		// 1. State inclusion: fold in_commit_i up the binary path.
		foldedState := merkleFold(api, c.InCommit[i], c.StatePath[i], c.StateDirs[i])
		api.AssertIsEqual(foldedState, c.StateRoots[i])

		// Reconstruct leaf_index_i from directions, LSB-first.
		leafIndex := bitsToField(api, c.StateDirs[i])

		// 2. Indexed non-inclusion: fold Poseidon2(low_value, next_value)
		//    up the nullifier tree path, assert low_value < nullifier < next_value.
		lowLeafHash := hashT3(api, c.NfLowValue[i], c.NfNextValue[i])
		foldedNull := merkleFold(api, lowLeafHash, c.NfLowPath[i], c.NfLowDirs[i])
		api.AssertIsEqual(foldedNull, c.NullifierRoots[i])

		// Strict range: low_value < nullifier < next_value.
		// AssertIsLessOrEqual(low+1, target) AND AssertIsLessOrEqual(target+1, next).
		// Use AssertIsDifferent for strict inequalities combined with <=.
		api.AssertIsLessOrEqual(api.Add(c.NfLowValue[i], 1), c.Nullifiers[i])
		api.AssertIsLessOrEqual(api.Add(c.Nullifiers[i], 1), c.NfNextValue[i])

		// 3. Binding: nullifier = Poseidon3(in_commit, leaf_index, dns).
		computedNf := hashT4(api, c.InCommit[i], leafIndex, c.DomainDNS[i])
		api.AssertIsEqual(computedNf, c.Nullifiers[i])
	}

	// Per-slot sub-chains, then a 3-element PublicInputsHash.
	stateRootsChain := hashChainCircuit(api, c.StateRoots)
	nullifierRootsChain := hashChainCircuit(api, c.NullifierRoots)
	nullifierChain := hashChainCircuit(api, c.Nullifiers)
	api.AssertIsEqual(
		hashChainCircuit(api, []frontend.Variable{
			stateRootsChain,
			nullifierRootsChain,
			nullifierChain,
		}),
		c.PublicInputsHash,
	)

	return nil
}

// merkleFold folds a leaf up a Merkle path using Poseidon2 at each level.
// directions[j] is asserted to be 0 or 1; 0 means the current hash is the
// LEFT child at level j, 1 means RIGHT.
func merkleFold(api frontend.API, leaf frontend.Variable, siblings []frontend.Variable, directions []frontend.Variable) frontend.Variable {
	if len(siblings) != len(directions) {
		panic(fmt.Sprintf("masp.merkleFold: siblings=%d directions=%d", len(siblings), len(directions)))
	}
	h := leaf
	for j := 0; j < len(siblings); j++ {
		api.AssertIsBoolean(directions[j])
		l := api.Select(directions[j], siblings[j], h)
		r := api.Select(directions[j], h, siblings[j])
		h = hashT3(api, l, r)
	}
	return h
}

// bitsToField assembles a sequence of bits (LSB-first) into a single field
// element Σ bit_j * 2^j. Each bit must already be constrained to {0, 1};
// merkleFold calls AssertIsBoolean for us.
func bitsToField(api frontend.API, bits []frontend.Variable) frontend.Variable {
	coeff := new(big.Int).SetUint64(1)
	two := big.NewInt(2)
	out := frontend.Variable(0)
	for _, b := range bits {
		out = api.Add(out, api.Mul(b, new(big.Int).Set(coeff)))
		coeff.Mul(coeff, two)
	}
	return out
}
