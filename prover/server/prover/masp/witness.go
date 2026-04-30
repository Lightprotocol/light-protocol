package masp

import (
	cryptoecdsa "crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"crypto/sha256"
	"fmt"
	"math/big"
	"sort"

	"github.com/consensys/gnark/std/math/emulated"
	gnarkecdsa "github.com/consensys/gnark/std/signature/ecdsa"
)

// Witness is a native representation of a MASP tx: all fields from which
// both circuits' witnesses are derived. One-shot: build this, then convert
// to the circuit-level assignments.
type Witness struct {
	N, M int

	// Per-input.
	InOwner     []*big.Int
	InSpl       []*big.Int
	InSol       []*big.Int
	InBlinding  []*big.Int
	InDataHash  []*big.Int
	InSeed      []*big.Int
	InProgramID []*big.Int
	InLeafIndex []*big.Int

	// Light compressed-account leaf binding for each input. TreeProof proves
	// inclusion of LightCompressedAccountLeafHash(..., InCommit), not direct
	// inclusion of InCommit.
	AccountOwnerHash     []*big.Int
	AccountTreeHash      []*big.Int
	AccountDiscriminator []*big.Int

	// Master spending secret.
	NullifierSecret *big.Int

	// Per-output.
	OutOwner             []*big.Int
	OutSpl               []*big.Int
	OutSol               []*big.Int
	OutBlinding          []*big.Int
	OutDataHash          []*big.Int
	OutOwnerIsProgram    []*big.Int
	OutOwnerProgramIndex []*big.Int
	OutSeed              []*big.Int

	TxBlinding *big.Int

	// Computed.
	InCommits          []*big.Int
	OutCommits         []*big.Int
	StateLeaves        []*big.Int
	DomainSecrets      []*big.Int
	Nullifiers         []*big.Int
	NullifierChain     *big.Int
	TxHash             *big.Int
	SeedsHashchain     *big.Int
	ProgramIDHashchain *big.Int
	// ShaTxHash = SHA-256(BE32(TxHash)) truncated to its top 31 bytes.
	// Fits in a single BN254 Fr field element (248 bits). The on-chain
	// Solana program is responsible for recomputing and asserting this
	// derivation before accepting the proof.
	ShaTxHash *big.Int

	// Public-inputs-hash folding: each circuit exposes one public input
	// (PublicInputsHash) formed by a Poseidon2 hash chain over its logical
	// public values. The on-chain verifier must recompute the same chain in
	// the same order before calling Groth16 verify.
	UtxoPublicInputsHash *big.Int
	TreePublicInputsHash *big.Int

	// P-256.
	PrivKey *cryptoecdsa.PrivateKey
	SigR    *big.Int
	SigS    *big.Int

	// Tree witnesses.
	StateRoot      *big.Int
	StateProofs    []StateTreeWitness // one per input
	NullifierRoot  *big.Int
	NonInclusionWs []NonInclusionWitness // one per input
}

// SampleMode selects the ownership mix of inputs when building a sample
// witness. Outputs follow a fixed pattern: output 0 is keypair-owned
// (free owner, data_hash = 0), outputs 1..M-1 are program-owned with a
// non-zero data_hash.
type SampleMode int

const (
	// SampleMixed: input 0 is keypair-owned (seed = 0); inputs 1..N-1 are
	// program-owned with seed = 0x1000+i.
	SampleMixed SampleMode = iota
	// SampleAllKeypair: every input is keypair-owned (seed = 0).
	SampleAllKeypair
	// SampleAllProgram: every input is program-owned (seed = 0x1000+i).
	SampleAllProgram
)

// NewSampleWitness builds a deterministic (up to P-256 keypair randomness)
// MASP witness for shape (N, M). Ownership follows the supplied SampleMode.
// All commitments, nullifiers, hashchains, signature, and tree witnesses
// are computed.
func NewSampleWitness(n, m int, mode SampleMode) (*Witness, error) {
	if n < 1 || m < 1 {
		return nil, fmt.Errorf("masp: shape (%d, %d) requires n>=1 and m>=1", n, m)
	}
	priv, err := cryptoecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	if err != nil {
		return nil, err
	}
	ownerKeypair := KeypairOwner(priv.PublicKey.X, priv.PublicKey.Y)

	w := &Witness{
		N:               n,
		M:               m,
		NullifierSecret: big.NewInt(0xB00B),
		TxBlinding:      big.NewInt(0xBAADF00D),
		PrivKey:         priv,
	}

	w.InOwner = make([]*big.Int, n)
	w.InSpl = make([]*big.Int, n)
	w.InSol = make([]*big.Int, n)
	w.InBlinding = make([]*big.Int, n)
	w.InDataHash = make([]*big.Int, n)
	w.InSeed = make([]*big.Int, n)
	w.InProgramID = make([]*big.Int, n)
	w.InLeafIndex = make([]*big.Int, n)
	w.AccountOwnerHash = make([]*big.Int, n)
	w.AccountTreeHash = make([]*big.Int, n)
	w.AccountDiscriminator = make([]*big.Int, n)

	// Flat amounts so value conservation (Σ in == Σ out) is trivial to satisfy.
	const splPerInput = 100
	const solPerInput = 1000
	for i := 0; i < n; i++ {
		w.InSpl[i] = big.NewInt(splPerInput)
		w.InSol[i] = big.NewInt(solPerInput)
		w.InBlinding[i] = big.NewInt(int64(111 + i))
		w.InDataHash[i] = big.NewInt(0)
		w.InProgramID[i] = big.NewInt(int64(0x10 + i))
		w.InLeafIndex[i] = big.NewInt(int64(1 + i*7))
		w.AccountOwnerHash[i] = SampleShieldedAccountOwnerHash()
		w.AccountTreeHash[i] = SampleUtxoTreeHash()
		w.AccountDiscriminator[i] = SampleShieldedUtxoDiscriminator()

		keypairOwned := false
		switch mode {
		case SampleAllKeypair:
			keypairOwned = true
		case SampleAllProgram:
			keypairOwned = false
		case SampleMixed:
			keypairOwned = (i == 0)
		}

		if keypairOwned {
			w.InSeed[i] = big.NewInt(0)
			w.InOwner[i] = ownerKeypair
		} else {
			seed := big.NewInt(int64(0x1000 + i))
			w.InSeed[i] = seed
			w.InOwner[i] = DomainNullifierSecret(w.InProgramID[i], seed) // == Poseidon2(pid, seed)
		}
	}

	w.OutOwner = make([]*big.Int, m)
	w.OutSpl = make([]*big.Int, m)
	w.OutSol = make([]*big.Int, m)
	w.OutBlinding = make([]*big.Int, m)
	w.OutDataHash = make([]*big.Int, m)
	w.OutOwnerIsProgram = make([]*big.Int, m)
	w.OutOwnerProgramIndex = make([]*big.Int, m)
	w.OutSeed = make([]*big.Int, m)

	// Distribute value: outputs 1..M-1 each get a flat unit; output 0 absorbs
	// the remainder so Σ in == Σ out holds for any (n, m).
	const splPerTailOutput = 1
	const solPerTailOutput = 1
	splRemainder := int64(n * splPerInput)
	solRemainder := int64(n * solPerInput)
	for j := 1; j < m; j++ {
		splRemainder -= splPerTailOutput
		solRemainder -= solPerTailOutput
	}
	for j := 0; j < m; j++ {
		w.OutBlinding[j] = big.NewInt(int64(333 + j))
		if j == 0 {
			w.OutSpl[j] = big.NewInt(splRemainder)
			w.OutSol[j] = big.NewInt(solRemainder)
			w.OutOwner[j] = ownerKeypair
			w.OutDataHash[j] = big.NewInt(0)
			w.OutOwnerIsProgram[j] = big.NewInt(0)
			w.OutOwnerProgramIndex[j] = big.NewInt(0)
			w.OutSeed[j] = big.NewInt(0)
		} else {
			w.OutSpl[j] = big.NewInt(splPerTailOutput)
			w.OutSol[j] = big.NewInt(solPerTailOutput)
			programIndex := j % n
			outSeed := big.NewInt(int64(0x5000 + j))
			w.OutOwner[j] = DomainNullifierSecret(w.InProgramID[programIndex], outSeed)
			w.OutDataHash[j] = big.NewInt(int64(0xBEEF + j))
			w.OutOwnerIsProgram[j] = big.NewInt(1)
			w.OutOwnerProgramIndex[j] = big.NewInt(int64(programIndex))
			w.OutSeed[j] = outSeed
		}
	}

	if err := w.Recompute(); err != nil {
		return nil, err
	}
	return w, nil
}

// Recompute derives all dependent fields (commitments, nullifiers,
// hashchains, tx_hash, signature, state/nullifier tree witnesses) from the
// current inputs and outputs. Call after mutating any field on a Witness so
// the derived state stays consistent.
func (w *Witness) Recompute() error {
	n, m := w.N, w.M
	w.ensureAccountBindingFields()

	w.InCommits = make([]*big.Int, n)
	w.StateLeaves = make([]*big.Int, n)
	w.DomainSecrets = make([]*big.Int, n)
	w.Nullifiers = make([]*big.Int, n)
	for i := 0; i < n; i++ {
		w.InCommits[i] = UtxoHash(Utxo{
			Owner: w.InOwner[i], Spl: w.InSpl[i], Sol: w.InSol[i],
			Blinding: w.InBlinding[i], DataHash: w.InDataHash[i],
		})
		w.StateLeaves[i] = LightCompressedAccountLeafHash(
			w.AccountOwnerHash[i],
			w.InLeafIndex[i],
			w.AccountTreeHash[i],
			w.AccountDiscriminator[i],
			w.InCommits[i],
		)
		// dns source: program_id if program-owned, else master secret.
		var source *big.Int
		if w.InSeed[i].Sign() == 0 {
			source = w.NullifierSecret
		} else {
			source = w.InProgramID[i]
		}
		w.DomainSecrets[i] = DomainNullifierSecret(w.InCommits[i], source)
		w.Nullifiers[i] = Nullify(w.InCommits[i], w.InLeafIndex[i], w.DomainSecrets[i])
	}
	w.OutCommits = make([]*big.Int, m)
	for j := 0; j < m; j++ {
		w.OutCommits[j] = UtxoHash(Utxo{
			Owner: w.OutOwner[j], Spl: w.OutSpl[j], Sol: w.OutSol[j],
			Blinding: w.OutBlinding[j], DataHash: w.OutDataHash[j],
		})
	}
	w.TxHash = TxHash(w.InCommits, w.Nullifiers, w.OutCommits, w.TxBlinding)
	w.NullifierChain = HashChain(w.Nullifiers)
	w.SeedsHashchain = HashChain(w.InSeed)
	w.ProgramIDHashchain = HashChain(w.InProgramID)

	// SHA-256 the BE32 encoding of TxHash, then truncate to 31 bytes so the
	// result fits in a single BN254 Fr field element. The signer signs these
	// 31 bytes directly (no further hashing). The on-chain Solana program
	// must perform the same truncation when checking the public input.
	var txHashBE [32]byte
	bs := w.TxHash.Bytes()
	copy(txHashBE[32-len(bs):], bs)
	shaDigest := sha256.Sum256(txHashBE[:])
	truncated := shaDigest[:31]
	w.ShaTxHash = new(big.Int).SetBytes(truncated)

	// When D is present we (re)sign here. Off-device/enclave reconstructions
	// arrive with D == nil and pre-populated SigR/SigS; trust them and only
	// re-verify.
	if w.PrivKey != nil && w.PrivKey.D != nil && w.PrivKey.D.Sign() != 0 {
		sigBin, err := w.PrivKey.Sign(rand.Reader, truncated, nil)
		if err != nil {
			return err
		}
		r, s, err := parseASN1SigRaw(sigBin)
		if err != nil {
			return err
		}
		w.SigR, w.SigS = r, s
	}
	if w.SigR != nil && w.SigS != nil && w.PrivKey != nil &&
		w.PrivKey.PublicKey.X != nil && w.PrivKey.PublicKey.Y != nil &&
		!cryptoecdsa.Verify(&w.PrivKey.PublicKey, truncated, w.SigR, w.SigS) {
		return fmt.Errorf("off-circuit P-256 verify failed")
	}

	// Sparse state tree: skip when caller has already populated StateRoot +
	// StateProofs (e.g. the client pre-computed them and shipped them via
	// WireWitness). BuildSparseStateTree dominates this function's runtime
	// (hundreds of Poseidon calls) so the guard is worth a few hundred ms.
	if w.StateRoot == nil || len(w.StateProofs) != n {
		entries := make(map[uint64]*big.Int, n)
		for i := 0; i < n; i++ {
			entries[w.InLeafIndex[i].Uint64()] = w.StateLeaves[i]
		}
		root, proofs := BuildSparseStateTree(entries)
		w.StateRoot = root
		w.StateProofs = make([]StateTreeWitness, n)
		for i := 0; i < n; i++ {
			w.StateProofs[i] = proofs[w.InLeafIndex[i].Uint64()]
		}
	}

	// Indexed nullifier tree: same guard.
	if w.NullifierRoot == nil || len(w.NonInclusionWs) != n {
		nfTree := NewIndexedTree()
		straddles := make([]*big.Int, 0, 2*n)
		for i := 0; i < n; i++ {
			straddles = append(straddles,
				new(big.Int).Sub(w.Nullifiers[i], big.NewInt(1)),
				new(big.Int).Add(w.Nullifiers[i], big.NewInt(1)),
			)
		}
		straddles = sortDedup(straddles)
		for _, v := range straddles {
			if v.Sign() <= 0 || v.Cmp(HighestAddressPlusOne) >= 0 {
				continue
			}
			nfTree.Insert(v)
		}
		w.NullifierRoot = nfTree.Root
		w.NonInclusionWs = make([]NonInclusionWitness, n)
		for i := 0; i < n; i++ {
			w.NonInclusionWs[i] = nfTree.NonInclusion(w.Nullifiers[i])
		}
	}

	// Fold each circuit's public inputs into a single field element. Order
	// must match the in-circuit computation.
	//
	// utxo_circuit: 5-element chain of fixed-length values. All variable-
	// length content (in/out commits, nullifiers) is absorbed into TxHash
	// (via in_nullifier_chain + out_commit_chain) and NullifierChain.
	w.UtxoPublicInputsHash = HashChain([]*big.Int{
		w.ShaTxHash,
		w.ProgramIDHashchain,
		w.SeedsHashchain,
		w.TxHash,
		w.NullifierChain,
	})

	// tree_circuit: fixed-length chain over sub-chains of the logical publics.
	stateRootsList := make([]*big.Int, n)
	nullifierRootsList := make([]*big.Int, n)
	for i := 0; i < n; i++ {
		stateRootsList[i] = w.StateRoot
		nullifierRootsList[i] = w.NullifierRoot
	}
	stateRootsChain := HashChain(stateRootsList)
	nullifierRootsChain := HashChain(nullifierRootsList)
	ownerHashChain := HashChain(w.AccountOwnerHash)
	treeHashChain := HashChain(w.AccountTreeHash)
	discriminatorChain := HashChain(w.AccountDiscriminator)
	w.TreePublicInputsHash = HashChain([]*big.Int{
		stateRootsChain,
		nullifierRootsChain,
		w.NullifierChain,
		ownerHashChain,
		treeHashChain,
		discriminatorChain,
	})
	return nil
}

func (w *Witness) ensureAccountBindingFields() {
	if len(w.AccountOwnerHash) != w.N {
		w.AccountOwnerHash = make([]*big.Int, w.N)
	}
	if len(w.AccountTreeHash) != w.N {
		w.AccountTreeHash = make([]*big.Int, w.N)
	}
	if len(w.AccountDiscriminator) != w.N {
		w.AccountDiscriminator = make([]*big.Int, w.N)
	}
	for i := 0; i < w.N; i++ {
		if w.AccountOwnerHash[i] == nil {
			w.AccountOwnerHash[i] = SampleShieldedAccountOwnerHash()
		}
		if w.AccountTreeHash[i] == nil {
			w.AccountTreeHash[i] = SampleUtxoTreeHash()
		}
		if w.AccountDiscriminator[i] == nil {
			w.AccountDiscriminator[i] = SampleShieldedUtxoDiscriminator()
		}
	}
}

func sortDedup(vs []*big.Int) []*big.Int {
	out := make([]*big.Int, 0, len(vs))
	for _, v := range vs {
		seen := false
		for _, o := range out {
			if o.Cmp(v) == 0 {
				seen = true
				break
			}
		}
		if !seen {
			out = append(out, new(big.Int).Set(v))
		}
	}
	sort.Slice(out, func(i, j int) bool { return out[i].Cmp(out[j]) < 0 })
	return out
}

func parseASN1SigRaw(sigBin []byte) (*big.Int, *big.Int, error) {
	r, s := new(big.Int), new(big.Int)
	if len(sigBin) < 2 || sigBin[0] != 0x30 {
		return nil, nil, fmt.Errorf("bad ASN.1 sig prefix")
	}
	cursor := sigBin[2:]
	if len(cursor) < 2 || cursor[0] != 0x02 {
		return nil, nil, fmt.Errorf("bad ASN.1 integer R")
	}
	rLen := int(cursor[1])
	r.SetBytes(cursor[2 : 2+rLen])
	cursor = cursor[2+rLen:]
	if len(cursor) < 2 || cursor[0] != 0x02 {
		return nil, nil, fmt.Errorf("bad ASN.1 integer S")
	}
	sLen := int(cursor[1])
	s.SetBytes(cursor[2 : 2+sLen])
	return r, s, nil
}

// UtxoAssignment returns the utxo_circuit assignment built from this witness.
func (w *Witness) UtxoAssignment() *UtxoCircuit {
	c := NewUtxoCircuit(w.N, w.M)
	for i := 0; i < w.N; i++ {
		c.InOwner[i] = w.InOwner[i]
		c.InSpl[i] = w.InSpl[i]
		c.InSol[i] = w.InSol[i]
		c.InBlinding[i] = w.InBlinding[i]
		c.InDataHash[i] = w.InDataHash[i]
		c.InSeed[i] = w.InSeed[i]
		c.InProgramID[i] = w.InProgramID[i]
		c.InLeafIndex[i] = w.InLeafIndex[i]
		c.Nullifiers[i] = w.Nullifiers[i]
	}
	c.NullifierSecret = w.NullifierSecret
	for j := 0; j < w.M; j++ {
		c.OutOwner[j] = w.OutOwner[j]
		c.OutSpl[j] = w.OutSpl[j]
		c.OutSol[j] = w.OutSol[j]
		c.OutBlinding[j] = w.OutBlinding[j]
		c.OutDataHash[j] = w.OutDataHash[j]
		c.OutOwnerIsProgram[j] = w.OutOwnerIsProgram[j]
		c.OutOwnerProgramIndex[j] = w.OutOwnerProgramIndex[j]
		c.OutSeed[j] = w.OutSeed[j]
		c.OutputCommitments[j] = w.OutCommits[j]
	}
	c.TxBlinding = w.TxBlinding
	c.TxHash = w.TxHash
	c.SeedsHashchain = w.SeedsHashchain
	c.ProgramIDHashchain = w.ProgramIDHashchain
	c.ShaTxHash = w.ShaTxHash
	c.NullifierChain = w.NullifierChain
	c.PublicInputsHash = w.UtxoPublicInputsHash
	c.Pub = gnarkecdsa.PublicKey[emulated.P256Fp, emulated.P256Fr]{
		X: emulated.ValueOf[emulated.P256Fp](w.PrivKey.PublicKey.X),
		Y: emulated.ValueOf[emulated.P256Fp](w.PrivKey.PublicKey.Y),
	}
	c.Sig = gnarkecdsa.Signature[emulated.P256Fr]{
		R: emulated.ValueOf[emulated.P256Fr](w.SigR),
		S: emulated.ValueOf[emulated.P256Fr](w.SigS),
	}
	return c
}

// TreeAssignment returns the tree_circuit assignment built from this witness.
func (w *Witness) TreeAssignment() *TreeCircuit {
	c := NewTreeCircuit(w.N)
	for i := 0; i < w.N; i++ {
		c.InCommit[i] = w.InCommits[i]
		c.AccountOwnerHash[i] = w.AccountOwnerHash[i]
		c.AccountTreeHash[i] = w.AccountTreeHash[i]
		c.AccountDiscriminator[i] = w.AccountDiscriminator[i]
		c.DomainDNS[i] = w.DomainSecrets[i]
		sp := w.StateProofs[i]
		for j := 0; j < StateTreeHeight; j++ {
			c.StatePath[i][j] = sp.Siblings[j]
			c.StateDirs[i][j] = sp.Directions[j]
		}
		nw := w.NonInclusionWs[i]
		c.NfLowValue[i] = nw.LowValue
		c.NfNextValue[i] = nw.NextValue
		for j := 0; j < IndexedTreeHeight; j++ {
			c.NfLowPath[i][j] = nw.Siblings[j]
			c.NfLowDirs[i][j] = nw.Directions[j]
		}
		c.StateRoots[i] = w.StateRoot
		c.NullifierRoots[i] = w.NullifierRoot
		c.Nullifiers[i] = w.Nullifiers[i]
	}
	c.PublicInputsHash = w.TreePublicInputsHash
	return c
}
