package masp

import (
	"encoding/json"
	"math/big"
	"testing"

	"light/light-prover/prover/common"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/test"
)

func TestUtxoAssignmentFromLocalWitnessSolves(t *testing.T) {
	w, err := NewSampleWitness(1, 1, SampleAllKeypair)
	if err != nil {
		t.Fatal(err)
	}

	assignment, err := UtxoAssignmentFromLocalWitness(
		uint32(w.N),
		uint32(w.M),
		bigString(w.UtxoPublicInputsHash),
		localUtxoWitnessFromWitness(w),
	)
	if err != nil {
		t.Fatal(err)
	}

	if err := test.IsSolved(NewUtxoCircuit(w.N, w.M), assignment, ecc.BN254.ScalarField()); err != nil {
		t.Fatalf("UTXO assignment from local witness should solve: %v", err)
	}
}

func TestTreeAssignmentFromLocalWitnessSolves(t *testing.T) {
	w, err := NewSampleWitness(1, 1, SampleAllKeypair)
	if err != nil {
		t.Fatal(err)
	}

	assignment, err := TreeAssignmentFromLocalWitness(
		uint32(w.N),
		bigString(w.TreePublicInputsHash),
		localTreeWitnessFromWitness(w),
	)
	if err != nil {
		t.Fatal(err)
	}

	if err := test.IsSolved(NewTreeCircuit(w.N), assignment, ecc.BN254.ScalarField()); err != nil {
		t.Fatalf("Tree assignment from local witness should solve: %v", err)
	}
}

func TestParseMaspUtxoProofRequestRequiresLocalWitness(t *testing.T) {
	body := []byte(`{"circuitType":"masp-utxo","nInputs":1,"nOutputs":1,"publicInputsHash":"1","rootContext":{}}`)

	if _, err := ParseMaspUtxoProofRequest(body); err == nil {
		t.Fatal("request without localWitness should fail")
	}
}

func TestParseMaspTreeProofRequest(t *testing.T) {
	w, err := NewSampleWitness(1, 1, SampleAllKeypair)
	if err != nil {
		t.Fatal(err)
	}
	req := MaspTreeProofRequest{
		MaspBaseRequest: MaspBaseRequest{
			CircuitType:      common.MaspTreeCircuitType,
			NInputs:          uint32(w.N),
			PublicInputsHash: bigString(w.TreePublicInputsHash),
			RootContext: RootContext{
				UtxoTreeID: "local-utxo-tree",
			},
		},
		LocalWitness: localTreeWitnessFromWitness(w),
	}
	body, err := json.Marshal(req)
	if err != nil {
		t.Fatal(err)
	}

	parsed, err := ParseMaspTreeProofRequest(body)
	if err != nil {
		t.Fatal(err)
	}
	if parsed.CircuitType != common.MaspTreeCircuitType || parsed.NInputs != 1 {
		t.Fatalf("unexpected parsed request: %+v", parsed.MaspBaseRequest)
	}
}

func TestProveUtxoRequestProducesGroth16Proof(t *testing.T) {
	w, err := NewSampleWitness(1, 1, SampleAllKeypair)
	if err != nil {
		t.Fatal(err)
	}
	km := common.NewLazyKeyManager(t.TempDir(), nil)
	RegisterDefaultBuilders(km)
	req := &MaspUtxoProofRequest{
		MaspBaseRequest: MaspBaseRequest{
			CircuitType:      common.MaspUtxoCircuitType,
			NInputs:          uint32(w.N),
			NOutputs:         uint32(w.M),
			PublicInputsHash: bigString(w.UtxoPublicInputsHash),
			RootContext: RootContext{
				UtxoTreeID: "local-utxo-tree",
			},
		},
		LocalWitness: localUtxoWitnessFromWitness(w),
	}

	proof, err := ProveUtxoRequest(km, req)
	if err != nil {
		t.Fatal(err)
	}
	if proof == nil || proof.Proof == nil {
		t.Fatal("expected Groth16 proof")
	}
}

func TestProveTreeRequestProducesGroth16Proof(t *testing.T) {
	w, err := NewSampleWitness(1, 1, SampleAllKeypair)
	if err != nil {
		t.Fatal(err)
	}
	km := common.NewLazyKeyManager(t.TempDir(), nil)
	RegisterDefaultBuilders(km)
	req := &MaspTreeProofRequest{
		MaspBaseRequest: MaspBaseRequest{
			CircuitType:      common.MaspTreeCircuitType,
			NInputs:          uint32(w.N),
			PublicInputsHash: bigString(w.TreePublicInputsHash),
			RootContext: RootContext{
				UtxoTreeID: "local-utxo-tree",
			},
		},
		LocalWitness: localTreeWitnessFromWitness(w),
	}

	proof, err := ProveTreeRequest(km, req)
	if err != nil {
		t.Fatal(err)
	}
	if proof == nil || proof.Proof == nil {
		t.Fatal("expected Groth16 proof")
	}
}

func localUtxoWitnessFromWitness(w *Witness) *MaspUtxoLocalWitness {
	return &MaspUtxoLocalWitness{
		InOwner:     bigStrings(w.InOwner),
		InSpl:       bigStrings(w.InSpl),
		InSol:       bigStrings(w.InSol),
		InBlinding:  bigStrings(w.InBlinding),
		InDataHash:  bigStrings(w.InDataHash),
		InSeed:      bigStrings(w.InSeed),
		InProgramID: bigStrings(w.InProgramID),
		InLeafIndex: bigStrings(w.InLeafIndex),

		NullifierSecret: bigString(w.NullifierSecret),

		OutOwner:             bigStrings(w.OutOwner),
		OutSpl:               bigStrings(w.OutSpl),
		OutSol:               bigStrings(w.OutSol),
		OutBlinding:          bigStrings(w.OutBlinding),
		OutDataHash:          bigStrings(w.OutDataHash),
		OutOwnerIsProgram:    bigStrings(w.OutOwnerIsProgram),
		OutOwnerProgramIndex: bigStrings(w.OutOwnerProgramIndex),
		OutSeed:              bigStrings(w.OutSeed),

		TxBlinding: bigString(w.TxBlinding),

		PubX: splitString(w.PrivKey.PublicKey.X),
		PubY: splitString(w.PrivKey.PublicKey.Y),
		SigR: splitString(w.SigR),
		SigS: splitString(w.SigS),

		Nullifiers:         bigStrings(w.Nullifiers),
		OutputCommitments:  bigStrings(w.OutCommits),
		TxHash:             bigString(w.TxHash),
		SeedsHashchain:     bigString(w.SeedsHashchain),
		ProgramIDHashchain: bigString(w.ProgramIDHashchain),
		ShaTxHash:          bigString(w.ShaTxHash),
		NullifierChain:     bigString(w.NullifierChain),
	}
}

func localTreeWitnessFromWitness(w *Witness) *MaspTreeLocalWitness {
	statePath := make([][]string, w.N)
	stateDirs := make([][]string, w.N)
	nfLowPath := make([][]string, w.N)
	nfLowDirs := make([][]string, w.N)
	for i := 0; i < w.N; i++ {
		statePath[i] = bigStrings(w.StateProofs[i].Siblings)
		stateDirs[i] = intStrings(w.StateProofs[i].Directions)
		nfLowPath[i] = bigStrings(w.NonInclusionWs[i].Siblings)
		nfLowDirs[i] = intStrings(w.NonInclusionWs[i].Directions)
	}
	stateRoots := make([]*big.Int, w.N)
	nullifierRoots := make([]*big.Int, w.N)
	for i := 0; i < w.N; i++ {
		stateRoots[i] = w.StateRoot
		nullifierRoots[i] = w.NullifierRoot
	}

	return &MaspTreeLocalWitness{
		InCommit:             bigStrings(w.InCommits),
		AccountOwnerHash:     bigStrings(w.AccountOwnerHash),
		AccountTreeHash:      bigStrings(w.AccountTreeHash),
		AccountDiscriminator: bigStrings(w.AccountDiscriminator),
		StatePath:            statePath,
		StateDirs:            stateDirs,
		DomainDNS:            bigStrings(w.DomainSecrets),
		NfLowValue:           nonInclusionLowValues(w),
		NfNextValue:          nonInclusionNextValues(w),
		NfLowPath:            nfLowPath,
		NfLowDirs:            nfLowDirs,
		StateRoots:           bigStrings(stateRoots),
		NullifierRoots:       bigStrings(nullifierRoots),
		Nullifiers:           bigStrings(w.Nullifiers),
	}
}

func nonInclusionLowValues(w *Witness) []string {
	out := make([]string, w.N)
	for i := 0; i < w.N; i++ {
		out[i] = bigString(w.NonInclusionWs[i].LowValue)
	}
	return out
}

func nonInclusionNextValues(w *Witness) []string {
	out := make([]string, w.N)
	for i := 0; i < w.N; i++ {
		out[i] = bigString(w.NonInclusionWs[i].NextValue)
	}
	return out
}

func splitString(v *big.Int) [2]string {
	hi, lo := splitHalves128(v)
	return [2]string{bigString(hi), bigString(lo)}
}

func bigStrings(values []*big.Int) []string {
	out := make([]string, len(values))
	for i, value := range values {
		out[i] = bigString(value)
	}
	return out
}

func intStrings(values []int) []string {
	out := make([]string, len(values))
	for i, value := range values {
		out[i] = big.NewInt(int64(value)).Text(10)
	}
	return out
}

func bigString(value *big.Int) string {
	return value.Text(10)
}
