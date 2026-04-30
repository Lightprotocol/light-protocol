package masp

import (
	"fmt"
	"math/big"

	"github.com/consensys/gnark/frontend"
)

// TreeCircuit proves:
//   - Per-input Light compressed-account hash is included in state_roots[i]
//     via a 40-level binary Merkle path.
//   - That compressed-account hash is the canonical no-lamports/no-address
//     Light account leaf whose data_hash is in_commit_i.
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
	InCommit             []frontend.Variable
	AccountOwnerHash     []frontend.Variable
	AccountTreeHash      []frontend.Variable
	AccountDiscriminator []frontend.Variable
	StatePath            [][]frontend.Variable // [N][height] siblings
	StateDirs            [][]frontend.Variable // [N][height] direction bits (0 = current is left, 1 = right)
	DomainDNS            []frontend.Variable   // per-input domain_nullifier_secret

	// Non-inclusion witnesses for nullifiers.
	NfLowValue  []frontend.Variable
	NfNextValue []frontend.Variable
	NfLowPath   [][]frontend.Variable // [N][height] siblings
	NfLowDirs   [][]frontend.Variable // [N][height] direction bits

	// ==== Logical "public" inputs, folded into PublicInputsHash ====
	StateRoots     []frontend.Variable
	NullifierRoots []frontend.Variable
	Nullifiers     []frontend.Variable

	// PublicInputsHash is the one real public input. It folds six
	// sub-chains (each of length N), in this order:
	//
	//   StateRootsChain     = HashChain(StateRoots[0..N-1])
	//   NullifierRootsChain = HashChain(NullifierRoots[0..N-1])
	//   NullifierChain      = HashChain(Nullifiers[0..N-1])
	//   OwnerHashChain      = HashChain(AccountOwnerHash[0..N-1])
	//   TreeHashChain       = HashChain(AccountTreeHash[0..N-1])
	//   DiscriminatorChain  = HashChain(AccountDiscriminator[0..N-1])
	//   PublicInputsHash    = HashChain([StateRootsChain, NullifierRootsChain,
	//                         NullifierChain, OwnerHashChain, TreeHashChain,
	//                         DiscriminatorChain])
	//
	// NullifierChain matches utxo_circuit's binding term — both circuits
	// compute it from their nullifier witnesses and the on-chain verifier
	// feeds the same digest into both Groth16 verifies. The account metadata
	// chains let the verifier bind the proof to the expected Light owner,
	// state tree, and shielded UTXO discriminator instead of accepting an
	// arbitrary compressed-account preimage.
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
	c.AccountOwnerHash = make([]frontend.Variable, n)
	c.AccountTreeHash = make([]frontend.Variable, n)
	c.AccountDiscriminator = make([]frontend.Variable, n)
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
		// Reconstruct leaf_index_i from directions, LSB-first.
		leafIndex := bitsToField(api, c.StateDirs[i])
		// Light compressed-account hashes use u32 leaf indices.
		api.ToBinary(leafIndex, 32)
		// The discriminator witness is the raw 8-byte account discriminator.
		api.ToBinary(c.AccountDiscriminator[i], 64)

		// 1. State inclusion: fold the Light compressed-account hash up the
		//    binary path. The account data_hash is the MASP UTXO commitment.
		stateLeaf := lightCompressedAccountLeafCircuit(
			api,
			c.AccountOwnerHash[i],
			leafIndex,
			c.AccountTreeHash[i],
			c.AccountDiscriminator[i],
			c.InCommit[i],
		)
		foldedState := merkleFold(api, stateLeaf, c.StatePath[i], c.StateDirs[i])
		api.AssertIsEqual(foldedState, c.StateRoots[i])

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

	// Per-slot sub-chains, then a fixed-length PublicInputsHash.
	stateRootsChain := hashChainCircuit(api, c.StateRoots)
	nullifierRootsChain := hashChainCircuit(api, c.NullifierRoots)
	nullifierChain := hashChainCircuit(api, c.Nullifiers)
	ownerHashChain := hashChainCircuit(api, c.AccountOwnerHash)
	treeHashChain := hashChainCircuit(api, c.AccountTreeHash)
	discriminatorChain := hashChainCircuit(api, c.AccountDiscriminator)
	api.AssertIsEqual(
		hashChainCircuit(api, []frontend.Variable{
			stateRootsChain,
			nullifierRootsChain,
			nullifierChain,
			ownerHashChain,
			treeHashChain,
			discriminatorChain,
		}),
		c.PublicInputsHash,
	)

	return nil
}

func lightCompressedAccountLeafCircuit(
	api frontend.API,
	ownerHash frontend.Variable,
	leafIndex frontend.Variable,
	treeHash frontend.Variable,
	discriminator frontend.Variable,
	utxoCommit frontend.Variable,
) frontend.Variable {
	discriminatorField := api.Add(discriminator, lightAccountDataDiscriminatorDomain)
	return hashT6(api, ownerHash, leafIndex, treeHash, discriminatorField, utxoCommit)
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
