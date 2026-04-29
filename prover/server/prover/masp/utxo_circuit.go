package masp

import (
	"fmt"

	"light/light-prover/prover/masp/poseidon"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/std/algebra/emulated/sw_emulated"
	"github.com/consensys/gnark/std/math/emulated"
	gnarkecdsa "github.com/consensys/gnark/std/signature/ecdsa"
)

// UtxoCircuit proves:
//   - Input UTXO commitments (in_commit_i) derive correctly from their fields.
//   - Per-input nullifier = Poseidon3(in_commit, leaf_index, dns) matches the
//     public Nullifiers slice, where dns is Poseidon2(in_commit, program_id)
//     for program-owned inputs (seed != 0) and Poseidon2(in_commit, master_secret)
//     for keypair-owned inputs (seed == 0). The user's master secret is never
//     mixed into program-owned nullifiers.
//   - All public nullifiers within one tx are pairwise distinct (rules out
//     self-double-spend inside the proof itself; the nullifier tree would
//     catch it later, but asserting here keeps a malformed proof from being
//     produced in the first place).
//   - Per-input owner matches Poseidon2(program_id, seed) when seed != 0,
//     else Poseidon4(pubkey.X_hi, X_lo, Y_hi, Y_lo).
//   - Output UTXO commitments recomputed from fields match OutputCommitments.
//   - For program-owned outputs: owner = Poseidon2(program_id_lookup, out_seed).
//   - For non-program-owned outputs: data_hash == 0 (owner is free).
//   - TxHash, SeedsHashchain, ProgramIDHashchain recomputed from witness match.
//   - P-256 signature verifies over ShaTxHash whenever at least one input has
//     seed == 0.
//   - Value conservation per asset: Σ InSpl == Σ OutSpl and
//     Σ InSol == Σ OutSol. Every amount is range-checked to 64 bits so field
//     wraparound cannot be used to fake balance.
type UtxoCircuit struct {
	NInputs  int `gnark:"-"`
	NOutputs int `gnark:"-"`

	// Per-input private witness.
	InOwner     []frontend.Variable
	InSpl       []frontend.Variable
	InSol       []frontend.Variable
	InBlinding  []frontend.Variable
	InDataHash  []frontend.Variable
	InSeed      []frontend.Variable
	InProgramID []frontend.Variable
	InLeafIndex []frontend.Variable

	// Master spending secret (single, shared across all inputs).
	NullifierSecret frontend.Variable

	// Per-output private witness.
	OutOwner    []frontend.Variable
	OutSpl      []frontend.Variable
	OutSol      []frontend.Variable
	OutBlinding []frontend.Variable
	OutDataHash []frontend.Variable
	// 1 iff this output is owned by a program that signed this tx.
	OutOwnerIsProgram []frontend.Variable
	// Index into InProgramID (0..NInputs-1). Unused when OutOwnerIsProgram==0.
	OutOwnerProgramIndex []frontend.Variable
	// out_seed_j: hashed with the selected program_id to derive the program-owned
	// output owner. Ignored when OutOwnerIsProgram==0.
	OutSeed []frontend.Variable

	// tx_hash blinding term (the final element in the tx_hash chain).
	TxBlinding frontend.Variable

	// P-256 public key and signature. Secret — revealed only indirectly via
	// owner_keypair.
	Pub gnarkecdsa.PublicKey[emulated.P256Fp, emulated.P256Fr]
	Sig gnarkecdsa.Signature[emulated.P256Fr]

	// ==== Logical "public" inputs, folded into PublicInputsHash ====
	// They are declared as private witnesses; the circuit hashes them into
	// PublicInputsHash and only that single value is actually public. This
	// shrinks the Groth16 public-input vector on-chain (one field element
	// total) while preserving the binding to every logical public value.
	Nullifiers         []frontend.Variable
	OutputCommitments  []frontend.Variable
	TxHash             frontend.Variable
	SeedsHashchain     frontend.Variable
	ProgramIDHashchain frontend.Variable
	// ShaTxHash: SHA-256(BE32(TxHash)) truncated to 31 bytes; used as the
	// P-256 ECDSA message. See README for the on-chain derivation contract.
	ShaTxHash frontend.Variable
	// NullifierChain = HashChain(Nullifiers[0..N-1]); cross-circuit binding
	// term. Computed in-circuit from the nullifier witnesses and asserted
	// against this value. The on-chain verifier recomputes the same chain
	// from the public nullifier list and feeds the digest into both circuit
	// verifications.
	NullifierChain frontend.Variable

	// PublicInputsHash is the one real public input: a Poseidon2 right-fold
	// over the logical inputs in this fixed order (low → high index):
	//
	//   ShaTxHash,
	//   ProgramIDHashchain,
	//   SeedsHashchain,
	//   TxHash,
	//   OutputCommitments[0..M-1],
	//   Nullifiers[0..N-1]
	//
	// Variable-length blocks (OutputCommitments, Nullifiers) live at the
	// high-index tail so that the on-chain verifier can precompute a small
	// table of "hash of D dummy nullifiers" / "hash of D dummy commits" and
	// apply the real slots on top per tx. Right-fold places the high-index
	// end at the deepest position of the hash chain, which is exactly the
	// cacheable partial-hash frontier.
	//
	// The on-chain verifier reconstructs the same chain off-circuit and
	// passes this digest to Groth16 verify. The circuit asserts it matches
	// what it computed from its private witnesses.
	PublicInputsHash frontend.Variable `gnark:",public"`
}

// NewUtxoCircuit returns a compile-ready circuit skeleton for shape (n, m).
// Slice fields are allocated so reflection tags resolve to the right lengths.
func NewUtxoCircuit(n, m int) *UtxoCircuit {
	if n < 1 || m < 1 {
		panic(fmt.Sprintf("masp.NewUtxoCircuit: n and m must be >= 1, got (%d, %d)", n, m))
	}
	c := &UtxoCircuit{NInputs: n, NOutputs: m}
	c.InOwner = make([]frontend.Variable, n)
	c.InSpl = make([]frontend.Variable, n)
	c.InSol = make([]frontend.Variable, n)
	c.InBlinding = make([]frontend.Variable, n)
	c.InDataHash = make([]frontend.Variable, n)
	c.InSeed = make([]frontend.Variable, n)
	c.InProgramID = make([]frontend.Variable, n)
	c.InLeafIndex = make([]frontend.Variable, n)
	c.OutOwner = make([]frontend.Variable, m)
	c.OutSpl = make([]frontend.Variable, m)
	c.OutSol = make([]frontend.Variable, m)
	c.OutBlinding = make([]frontend.Variable, m)
	c.OutDataHash = make([]frontend.Variable, m)
	c.OutOwnerIsProgram = make([]frontend.Variable, m)
	c.OutOwnerProgramIndex = make([]frontend.Variable, m)
	c.OutSeed = make([]frontend.Variable, m)
	c.Nullifiers = make([]frontend.Variable, n)
	c.OutputCommitments = make([]frontend.Variable, m)
	return c
}

func (c *UtxoCircuit) Define(api frontend.API) error {
	if c.NInputs < 1 || c.NOutputs < 1 {
		return fmt.Errorf("masp.UtxoCircuit: NInputs and NOutputs must be set to positive values")
	}

	// Owner from keypair: split each 256-bit coordinate into two 128-bit
	// halves and hash all four with Poseidon4.
	ownerKeypair := keypairOwnerCircuit(api, c.Pub)

	// Track "any input uses seed == 0" so the signature check can be gated.
	// seedNonZero_i = 1 - IsZero(seed_i). any_keypair_input = 1 - (prod of seedNonZero).
	prodNonZero := frontend.Variable(1)

	inCommits := make([]frontend.Variable, c.NInputs)

	// Accumulators for value conservation.
	sumInSpl := frontend.Variable(0)
	sumInSol := frontend.Variable(0)

	for i := 0; i < c.NInputs; i++ {
		// Range-check amounts to 64 bits so field wraparound cannot be used
		// to fake value balance (N · 2^64 ≪ BN254 Fr modulus).
		api.ToBinary(c.InSpl[i], 64)
		api.ToBinary(c.InSol[i], 64)
		sumInSpl = api.Add(sumInSpl, c.InSpl[i])
		sumInSol = api.Add(sumInSol, c.InSol[i])

		// in_commit = Poseidon6(0, owner, spl, sol, blinding, data_hash)
		inCommit := hashT7(api,
			frontend.Variable(0),
			c.InOwner[i], c.InSpl[i], c.InSol[i], c.InBlinding[i], c.InDataHash[i],
		)
		inCommits[i] = inCommit

		// Owner reconstruction.
		seedIsZero := api.IsZero(c.InSeed[i])
		seedNonZero := api.Sub(1, seedIsZero)
		prodNonZero = api.Mul(prodNonZero, seedNonZero)

		ownerProgram := hashT3(api, c.InProgramID[i], c.InSeed[i])
		reconstructedOwner := api.Select(seedNonZero, ownerProgram, ownerKeypair)
		api.AssertIsEqual(reconstructedOwner, c.InOwner[i])

		// dns source: program-owned inputs mix in program_id (keeping the
		// user's master secret out of program-controlled spends);
		// keypair-owned inputs mix in the master secret.
		//
		// NOTE: the circuit treats InProgramID[i] as a raw witness — it does
		// NOT prove that this program_id corresponds to a program that
		// actually signed / invoked this tx. The on-chain verifying program
		// is responsible for enforcing, via the CPI context account, that
		// the ProgramIDHashchain public input matches the chain of invoking
		// program_ids on the call stack. Without that external check, a
		// prover could claim any program_id for a program-owned input and
		// produce a valid-looking nullifier.
		dnsSource := api.Select(seedNonZero, c.InProgramID[i], c.NullifierSecret)
		dns := hashT3(api, inCommit, dnsSource)
		nf := hashT4(api, inCommit, c.InLeafIndex[i], dns)
		api.AssertIsEqual(nf, c.Nullifiers[i])
	}

	// Pairwise-distinct nullifiers inside this tx.
	// O(N^2/2) IsZero checks; cheap for the shapes we bench.
	for i := 0; i < c.NInputs; i++ {
		for j := i + 1; j < c.NInputs; j++ {
			same := api.IsZero(api.Sub(c.Nullifiers[i], c.Nullifiers[j]))
			api.AssertIsEqual(same, 0)
		}
	}

	// any_keypair_input = 1 iff prodNonZero == 0. Equivalently IsZero(prodNonZero).
	anyKeypairInput := api.IsZero(prodNonZero)

	outCommits := make([]frontend.Variable, c.NOutputs)
	sumOutSpl := frontend.Variable(0)
	sumOutSol := frontend.Variable(0)
	for j := 0; j < c.NOutputs; j++ {
		// Range-check output amounts to 64 bits.
		api.ToBinary(c.OutSpl[j], 64)
		api.ToBinary(c.OutSol[j], 64)
		sumOutSpl = api.Add(sumOutSpl, c.OutSpl[j])
		sumOutSol = api.Add(sumOutSol, c.OutSol[j])

		outCommit := hashT7(api,
			frontend.Variable(0),
			c.OutOwner[j], c.OutSpl[j], c.OutSol[j], c.OutBlinding[j], c.OutDataHash[j],
		)
		outCommits[j] = outCommit
		api.AssertIsEqual(outCommit, c.OutputCommitments[j])

		// data_hash gating.
		api.AssertIsBoolean(c.OutOwnerIsProgram[j])
		// If OutOwnerIsProgram_j == 0: data_hash_j must be 0.
		// Equivalently: (1 - OutOwnerIsProgram_j) * data_hash_j == 0.
		api.AssertIsEqual(api.Mul(api.Sub(1, c.OutOwnerIsProgram[j]), c.OutDataHash[j]), 0)

		// If OutOwnerIsProgram_j == 1: owner_j == Poseidon2(program_id_lookup, out_seed_j).
		// Program-id lookup: demux OutOwnerProgramIndex over the N program ids.
		pid, idxValid := demuxSelect(api, c.OutOwnerProgramIndex[j], c.InProgramID)
		// When this output is program-owned, the index must be in [0, N).
		// Soundness: without this, the prover can pick an out-of-range index,
		// forcing pid=0 and a fabricated owner, and unlock unrestricted data_hash.
		api.AssertIsEqual(api.Mul(c.OutOwnerIsProgram[j], api.Sub(1, idxValid)), 0)
		expectedProgramOwner := hashT3(api, pid, c.OutSeed[j])
		// Assert: OutOwnerIsProgram_j * (OutOwner_j - expectedProgramOwner) == 0.
		api.AssertIsEqual(api.Mul(c.OutOwnerIsProgram[j], api.Sub(c.OutOwner[j], expectedProgramOwner)), 0)
	}

	// Value conservation per asset.
	api.AssertIsEqual(sumInSpl, sumOutSpl)
	api.AssertIsEqual(sumInSol, sumOutSol)

	// Hashchains.
	txHashComputed := txHashCircuit(api, inCommits, c.Nullifiers, outCommits, c.TxBlinding)
	api.AssertIsEqual(txHashComputed, c.TxHash)

	seedsHashChainComputed := hashChainCircuit(api, c.InSeed)
	api.AssertIsEqual(seedsHashChainComputed, c.SeedsHashchain)

	programIDHashChainComputed := hashChainCircuit(api, c.InProgramID)
	api.AssertIsEqual(programIDHashChainComputed, c.ProgramIDHashchain)

	// Conditional P-256 signature verification.
	// Message is sha_tx_hash (SHA-256(BE32(tx_hash)) truncated to 31 bytes),
	// reinterpreted as a P-256 Fr scalar.
	msg := shaTxHashToP256Fr(api, c.ShaTxHash)
	sigValidBit := c.Pub.IsValid(api, sw_emulated.GetCurveParams[emulated.P256Fp](), msg, &c.Sig)
	// Assert: sigValid OR NOT any_keypair_input.
	// i.e. anyKeypairInput * (1 - sigValid) == 0.
	api.AssertIsEqual(api.Mul(anyKeypairInput, api.Sub(1, sigValidBit)), 0)

	// NullifierChain binds this circuit to tree_circuit: both compute it
	// from their nullifier witnesses; the on-chain verifier reconstructs
	// from the known nullifier values and passes the same digest to both.
	nullifierChainComputed := hashChainCircuit(api, c.Nullifiers)
	api.AssertIsEqual(nullifierChainComputed, c.NullifierChain)

	// Five-element PublicInputsHash. Order must match the on-chain
	// verifier's reconstruction.
	api.AssertIsEqual(
		hashChainCircuit(api, []frontend.Variable{
			c.ShaTxHash,
			c.ProgramIDHashchain,
			c.SeedsHashchain,
			c.TxHash,
			c.NullifierChain,
		}),
		c.PublicInputsHash,
	)

	return nil
}

func hashT3(api frontend.API, a, b frontend.Variable) frontend.Variable {
	h := poseidon.HashCircuitWithT(api, 3, []frontend.Variable{a, b})
	return h
}

func hashT4(api frontend.API, a, b, c frontend.Variable) frontend.Variable {
	return poseidon.HashCircuitWithT(api, 4, []frontend.Variable{a, b, c})
}

func hashT5(api frontend.API, a, b, c, d frontend.Variable) frontend.Variable {
	return poseidon.HashCircuitWithT(api, 5, []frontend.Variable{a, b, c, d})
}

func hashT7(api frontend.API, a, b, c, d, e, f frontend.Variable) frontend.Variable {
	return poseidon.HashCircuitWithT(api, 7, []frontend.Variable{a, b, c, d, e, f})
}

// hashChainCircuit is the in-circuit analog of native HashChain. Right-fold,
// walked high → low so a suffix of the input list is a precomputable
// partial hash:
//
//	h = inputs[N-1]
//	for i := N-2; i >= 0; i--:
//	    h = Poseidon2(inputs[i], h)
func hashChainCircuit(api frontend.API, inputs []frontend.Variable) frontend.Variable {
	n := len(inputs)
	if n == 0 {
		return frontend.Variable(0)
	}
	h := inputs[n-1]
	for i := n - 2; i >= 0; i-- {
		h = hashT3(api, inputs[i], h)
	}
	return h
}

func inNullifierChainCircuit(api frontend.API, inCommits, nullifiers []frontend.Variable) frontend.Variable {
	if len(inCommits) != len(nullifiers) {
		panic("masp: inNullifierChainCircuit length mismatch")
	}
	interleaved := make([]frontend.Variable, 0, 2*len(inCommits))
	for i := range inCommits {
		interleaved = append(interleaved, inCommits[i], nullifiers[i])
	}
	return hashChainCircuit(api, interleaved)
}

func txHashCircuit(api frontend.API, inCommits, nullifiers, outCommits []frontend.Variable, blinding frontend.Variable) frontend.Variable {
	inChain := inNullifierChainCircuit(api, inCommits, nullifiers)
	outChain := hashChainCircuit(api, outCommits)
	return hashChainCircuit(api, []frontend.Variable{
		frontend.Variable(1), // DomainTxHash
		blinding,
		inChain,
		outChain,
	})
}

// keypairOwnerCircuit derives a BN254-Fr owner from a P-256 public key by
// splitting each 256-bit coordinate into two 128-bit halves and hashing the
// four halves with Poseidon4. Each half is < 2^128, comfortably below the
// BN254 Fr modulus (~2^254).
func keypairOwnerCircuit(api frontend.API, pub gnarkecdsa.PublicKey[emulated.P256Fp, emulated.P256Fr]) frontend.Variable {
	fp, err := emulated.NewField[emulated.P256Fp](api)
	if err != nil {
		panic(err)
	}
	xBits := fp.ToBits(&pub.X)
	yBits := fp.ToBits(&pub.Y)
	if len(xBits) < 256 || len(yBits) < 256 {
		panic(fmt.Sprintf("masp.keypairOwnerCircuit: expected >=256 bits per coordinate, got x=%d y=%d", len(xBits), len(yBits)))
	}
	xLo := api.FromBinary(xBits[0:128]...)
	xHi := api.FromBinary(xBits[128:256]...)
	yLo := api.FromBinary(yBits[0:128]...)
	yHi := api.FromBinary(yBits[128:256]...)
	return hashT5(api, xHi, xLo, yHi, yLo)
}

// shaTxHashToP256Fr reinterprets sha_tx_hash (a BN254 Fr element that fits
// in 248 bits — the on-chain program truncates SHA-256 to 31 bytes before
// passing it in) as a P-256 Fr scalar.
//
// `api.ToBinary(·, 248)` asserts the value fits in 248 bits.
func shaTxHashToP256Fr(api frontend.API, shaTxHash frontend.Variable) *emulated.Element[emulated.P256Fr] {
	bits := api.ToBinary(shaTxHash, 248)
	// Pad to 256 bits with zeros.
	padded := make([]frontend.Variable, 256)
	copy(padded, bits)
	for i := 248; i < 256; i++ {
		padded[i] = frontend.Variable(0)
	}

	fr, err := emulated.NewField[emulated.P256Fr](api)
	if err != nil {
		panic(err)
	}
	return fr.FromBits(padded...)
}

// demuxSelect returns (inputs[idx], idxValid) where idxValid is a bit that
// is 1 iff idx ∈ [0, len(inputs)). When idxValid == 0 the returned value is
// 0, but callers must not rely on it — they must gate their use of the
// result on idxValid == 1.
//
//	out      = Σ (idx == k) * inputs[k]   for k in [0, N)
//	idxValid = Σ (idx == k)                for k in [0, N)
//
// For N ≤ 32 this is linear and cheap (one IsZero + one Mul + one Add per entry).
func demuxSelect(api frontend.API, idx frontend.Variable, inputs []frontend.Variable) (frontend.Variable, frontend.Variable) {
	out := frontend.Variable(0)
	idxValid := frontend.Variable(0)
	for k := 0; k < len(inputs); k++ {
		eq := api.IsZero(api.Sub(idx, k))
		out = api.Add(out, api.Mul(eq, inputs[k]))
		idxValid = api.Add(idxValid, eq)
	}
	return out, idxValid
}
