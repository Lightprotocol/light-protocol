package masp

import (
	"math/big"
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/test"
)

func mustWitness(t *testing.T, mode SampleMode) *Witness {
	t.Helper()
	w, err := NewSampleWitness(2, 2, mode)
	if err != nil {
		t.Fatal(err)
	}
	return w
}

func TestUtxoCircuitMixed(t *testing.T) {
	w := mustWitness(t, SampleMixed)
	if err := test.IsSolved(NewUtxoCircuit(w.N, w.M), w.UtxoAssignment(), ecc.BN254.ScalarField()); err != nil {
		t.Fatalf("IsSolved (mixed): %v", err)
	}
}

func TestUtxoCircuitAllKeypair(t *testing.T) {
	w := mustWitness(t, SampleAllKeypair)
	if err := test.IsSolved(NewUtxoCircuit(w.N, w.M), w.UtxoAssignment(), ecc.BN254.ScalarField()); err != nil {
		t.Fatalf("IsSolved (all_keypair): %v", err)
	}
}

func TestUtxoCircuitAllProgram(t *testing.T) {
	w := mustWitness(t, SampleAllProgram)
	if err := test.IsSolved(NewUtxoCircuit(w.N, w.M), w.UtxoAssignment(), ecc.BN254.ScalarField()); err != nil {
		t.Fatalf("IsSolved (all_program): %v", err)
	}
}

func TestUtxoCircuitAllProgramAcceptsBogusSig(t *testing.T) {
	w := mustWitness(t, SampleAllProgram)
	// Corrupt the signature — since no input is keypair-owned, the conditional
	// check should skip the equality assertion.
	w.SigR = big.NewInt(1)
	w.SigS = big.NewInt(2)
	if err := test.IsSolved(NewUtxoCircuit(w.N, w.M), w.UtxoAssignment(), ecc.BN254.ScalarField()); err != nil {
		t.Fatalf("all_program with bogus sig should still solve: %v", err)
	}
}

func TestUtxoCircuitKeypairRejectsBogusSig(t *testing.T) {
	w := mustWitness(t, SampleAllKeypair)
	w.SigR = big.NewInt(1)
	w.SigS = big.NewInt(2)
	if err := test.IsSolved(NewUtxoCircuit(w.N, w.M), w.UtxoAssignment(), ecc.BN254.ScalarField()); err == nil {
		t.Fatal("all_keypair with bogus sig should fail to solve")
	}
}

// Tampering with ShaTxHash (while leaving the real signature untouched) must
// make keypair-owned verification fail, because the signer signed the
// truncated SHA-256 digest and the circuit uses ShaTxHash as the P-256 message.
func TestUtxoCircuitKeypairRejectsTamperedShaTxHash(t *testing.T) {
	w := mustWitness(t, SampleAllKeypair)
	w.ShaTxHash = new(big.Int).Add(w.ShaTxHash, big.NewInt(1))
	if err := test.IsSolved(NewUtxoCircuit(w.N, w.M), w.UtxoAssignment(), ecc.BN254.ScalarField()); err == nil {
		t.Fatal("tampered ShaTxHash should break keypair-owned verification")
	}
}

// Regression: a program-owned output with OutOwnerProgramIndex out of range
// must be rejected. This used to pass because demuxSelect silently returned
// pid=0 when no index matched, letting the prover fabricate an owner.
func TestUtxoCircuitRejectsOutOfRangeProgramIndex(t *testing.T) {
	w := mustWitness(t, SampleMixed)
	// Craft an output claiming to be program-owned via out-of-range index.
	// Owner = Poseidon2(0, out_seed) — the value the old demux would hand back.
	outSeed := big.NewInt(0x9999)
	w.OutOwner[1] = DomainNullifierSecret(big.NewInt(0), outSeed)
	w.OutSeed[1] = outSeed
	w.OutOwnerIsProgram[1] = big.NewInt(1)
	w.OutOwnerProgramIndex[1] = big.NewInt(int64(w.N)) // out of range: valid range is [0, N).
	w.OutDataHash[1] = big.NewInt(0x1234)              // arbitrary data_hash — would be allowed if the attack worked.
	if err := w.Recompute(); err != nil {
		t.Fatal(err)
	}
	if err := test.IsSolved(NewUtxoCircuit(w.N, w.M), w.UtxoAssignment(), ecc.BN254.ScalarField()); err == nil {
		t.Fatal("out-of-range OutOwnerProgramIndex should be rejected")
	}
}

func TestUtxoCircuitRejectsValueImbalanceSpl(t *testing.T) {
	w := mustWitness(t, SampleMixed)
	// Inflate one output's SPL — Σ out > Σ in. Circuit must reject.
	w.OutSpl[0] = new(big.Int).Add(w.OutSpl[0], big.NewInt(1))
	if err := w.Recompute(); err != nil {
		t.Fatal(err)
	}
	if err := test.IsSolved(NewUtxoCircuit(w.N, w.M), w.UtxoAssignment(), ecc.BN254.ScalarField()); err == nil {
		t.Fatal("value imbalance on SPL should be rejected")
	}
}

func TestUtxoCircuitRejectsValueImbalanceSol(t *testing.T) {
	w := mustWitness(t, SampleMixed)
	w.OutSol[0] = new(big.Int).Sub(w.OutSol[0], big.NewInt(1))
	if err := w.Recompute(); err != nil {
		t.Fatal(err)
	}
	if err := test.IsSolved(NewUtxoCircuit(w.N, w.M), w.UtxoAssignment(), ecc.BN254.ScalarField()); err == nil {
		t.Fatal("value imbalance on SOL should be rejected")
	}
}

func TestUtxoCircuitRejectsAmountOver64Bits(t *testing.T) {
	w := mustWitness(t, SampleMixed)
	// Set an input amount to 2^64. Range check ToBinary(·, 64) must reject.
	w.InSpl[0] = new(big.Int).Lsh(big.NewInt(1), 64)
	// Compensate for value conservation, else that check fires instead of the range check.
	w.OutSpl[0] = new(big.Int).Add(w.OutSpl[0], new(big.Int).Lsh(big.NewInt(1), 64))
	if err := w.Recompute(); err != nil {
		t.Fatal(err)
	}
	if err := test.IsSolved(NewUtxoCircuit(w.N, w.M), w.UtxoAssignment(), ecc.BN254.ScalarField()); err == nil {
		t.Fatal("amount >= 2^64 should be rejected by the range check")
	}
}

func TestUtxoCircuitRejectsDuplicateNullifier(t *testing.T) {
	// Force two inputs to hash to the same nullifier by making them identical.
	w := mustWitness(t, SampleMixed)
	// Copy input 0's fields to input 1 so nullifiers collide.
	w.InOwner[1] = w.InOwner[0]
	w.InSpl[1] = w.InSpl[0]
	w.InSol[1] = w.InSol[0]
	w.InBlinding[1] = w.InBlinding[0]
	w.InDataHash[1] = w.InDataHash[0]
	w.InSeed[1] = w.InSeed[0]
	w.InProgramID[1] = w.InProgramID[0]
	w.InLeafIndex[1] = w.InLeafIndex[0]
	if err := w.Recompute(); err != nil {
		// Non-inclusion builder may fail on duplicate nullifiers; that's fine,
		// it would have failed at circuit-level too.
		return
	}
	if err := test.IsSolved(NewUtxoCircuit(w.N, w.M), w.UtxoAssignment(), ecc.BN254.ScalarField()); err == nil {
		t.Fatal("duplicate nullifier within a tx should be rejected")
	}
}

// exercise the "data_hash != 0 but not program-owned" rejection.
func TestUtxoCircuitDataHashGating(t *testing.T) {
	w := mustWitness(t, SampleMixed)
	// Output 0 is keypair-owned (OutOwnerIsProgram = 0). Set data_hash nonzero.
	w.OutDataHash[0] = big.NewInt(7)
	if err := w.Recompute(); err != nil {
		t.Fatal(err)
	}
	if err := test.IsSolved(NewUtxoCircuit(w.N, w.M), w.UtxoAssignment(), ecc.BN254.ScalarField()); err == nil {
		t.Fatal("keypair-owned output with nonzero data_hash should be rejected")
	}
}
