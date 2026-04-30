package masp

import (
	cryptoecdsa "crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"encoding/json"
	"fmt"
	"math/big"
	"os"
	"testing"

	"light/light-prover/prover/common"
)

func TestExportLocalDevProofRequestFixtures(t *testing.T) {
	outputPath := os.Getenv("MASP_FIXTURE_OUT")
	if outputPath == "" {
		t.Skip("set MASP_FIXTURE_OUT to export local/dev MASP request fixtures")
	}

	if specPath := os.Getenv("MASP_ZONE_FIXTURE_SPEC"); specPath != "" {
		fixtures, err := exportLocalDevZoneProofRequests(specPath)
		if err != nil {
			t.Fatal(err)
		}
		writeFixtureOutput(t, outputPath, fixtures)
		return
	}

	w, err := NewSampleWitness(1, 1, SampleAllKeypair)
	if err != nil {
		t.Fatal(err)
	}
	fixtures := map[string]any{
		"utxo": MaspUtxoProofRequest{
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
		},
		"tree": MaspTreeProofRequest{
			MaspBaseRequest: MaspBaseRequest{
				CircuitType:      common.MaspTreeCircuitType,
				NInputs:          uint32(w.N),
				PublicInputsHash: bigString(w.TreePublicInputsHash),
				RootContext: RootContext{
					UtxoTreeID: "local-utxo-tree",
				},
			},
			LocalWitness: localTreeWitnessFromWitness(w),
		},
	}

	writeFixtureOutput(t, outputPath, fixtures)
}

type localDevZoneSpec struct {
	RootContext         RootContext              `json:"rootContext"`
	OperationCommitment string                   `json:"operationCommitment"`
	NullifierSecret     string                   `json:"nullifierSecret"`
	TxBlinding          string                   `json:"txBlinding"`
	Inputs              []localDevZoneInputSpec  `json:"inputs"`
	Outputs             []localDevZoneOutputSpec `json:"outputs"`
}

type localDevZoneInputSpec struct {
	Owner                     string   `json:"owner"`
	Spl                       string   `json:"spl"`
	Sol                       string   `json:"sol"`
	Blinding                  string   `json:"blinding"`
	DataHash                  string   `json:"dataHash"`
	Seed                      string   `json:"seed"`
	ProgramID                 string   `json:"programId"`
	LeafIndex                 string   `json:"leafIndex"`
	AccountOwnerHash          string   `json:"accountOwnerHash"`
	AccountTreeHash           string   `json:"accountTreeHash"`
	AccountDiscriminator      string   `json:"accountDiscriminator"`
	StateRoot                 string   `json:"stateRoot"`
	StatePath                 []string `json:"statePath"`
	StateDirs                 []int    `json:"stateDirs"`
	NullifierRoot             string   `json:"nullifierRoot"`
	NfLowValue                string   `json:"nfLowValue"`
	NfNextValue               string   `json:"nfNextValue"`
	NfLowPath                 []string `json:"nfLowPath"`
	NfLowDirs                 []int    `json:"nfLowDirs"`
	ExpectedUtxoHash          string   `json:"expectedUtxoHash"`
	ExpectedSpendNullifier    string   `json:"expectedSpendNullifier"`
	ExpectedCompressedAccount string   `json:"expectedCompressedAccountHash"`
}

type localDevZoneOutputSpec struct {
	Owner             string `json:"owner"`
	Spl               string `json:"spl"`
	Sol               string `json:"sol"`
	Blinding          string `json:"blinding"`
	DataHash          string `json:"dataHash"`
	OwnerIsProgram    string `json:"ownerIsProgram"`
	OwnerProgramIndex string `json:"ownerProgramIndex"`
	Seed              string `json:"seed"`
}

func exportLocalDevZoneProofRequests(specPath string) (map[string]any, error) {
	buf, err := os.ReadFile(specPath)
	if err != nil {
		return nil, err
	}
	var spec localDevZoneSpec
	if err := json.Unmarshal(buf, &spec); err != nil {
		return nil, fmt.Errorf("decode MASP zone fixture spec: %w", err)
	}
	w, err := witnessFromLocalDevZoneSpec(spec)
	if err != nil {
		return nil, err
	}
	return map[string]any{
		"utxo": MaspUtxoProofRequest{
			MaspBaseRequest: MaspBaseRequest{
				CircuitType:      common.MaspUtxoCircuitType,
				NInputs:          uint32(w.N),
				NOutputs:         uint32(w.M),
				PublicInputsHash: bigString(w.UtxoPublicInputsHash),
				RootContext:      spec.RootContext,
				OperationCommit:  spec.OperationCommitment,
			},
			LocalWitness: localUtxoWitnessFromWitness(w),
		},
		"tree": MaspTreeProofRequest{
			MaspBaseRequest: MaspBaseRequest{
				CircuitType:      common.MaspTreeCircuitType,
				NInputs:          uint32(w.N),
				PublicInputsHash: bigString(w.TreePublicInputsHash),
				RootContext:      spec.RootContext,
				OperationCommit:  spec.OperationCommitment,
			},
			LocalWitness: localTreeWitnessFromWitness(w),
		},
	}, nil
}

func witnessFromLocalDevZoneSpec(spec localDevZoneSpec) (*Witness, error) {
	if len(spec.Inputs) == 0 {
		return nil, fmt.Errorf("zone fixture requires at least one input")
	}
	if len(spec.Outputs) == 0 {
		return nil, fmt.Errorf("zone fixture requires at least one output")
	}
	priv, err := cryptoecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	if err != nil {
		return nil, err
	}
	w := &Witness{
		N:               len(spec.Inputs),
		M:               len(spec.Outputs),
		NullifierSecret: mustParseFixtureBig(spec.NullifierSecret, "nullifierSecret"),
		TxBlinding:      mustParseFixtureBig(spec.TxBlinding, "txBlinding"),
		PrivKey:         priv,
	}
	w.InOwner = make([]*big.Int, w.N)
	w.InSpl = make([]*big.Int, w.N)
	w.InSol = make([]*big.Int, w.N)
	w.InBlinding = make([]*big.Int, w.N)
	w.InDataHash = make([]*big.Int, w.N)
	w.InSeed = make([]*big.Int, w.N)
	w.InProgramID = make([]*big.Int, w.N)
	w.InLeafIndex = make([]*big.Int, w.N)
	w.AccountOwnerHash = make([]*big.Int, w.N)
	w.AccountTreeHash = make([]*big.Int, w.N)
	w.AccountDiscriminator = make([]*big.Int, w.N)
	w.StateProofs = make([]StateTreeWitness, w.N)
	w.NonInclusionWs = make([]NonInclusionWitness, w.N)

	for i, input := range spec.Inputs {
		w.InOwner[i] = mustParseFixtureBig(input.Owner, fmt.Sprintf("inputs[%d].owner", i))
		w.InSpl[i] = mustParseFixtureBig(input.Spl, fmt.Sprintf("inputs[%d].spl", i))
		w.InSol[i] = mustParseFixtureBig(input.Sol, fmt.Sprintf("inputs[%d].sol", i))
		w.InBlinding[i] = mustParseFixtureBig(input.Blinding, fmt.Sprintf("inputs[%d].blinding", i))
		w.InDataHash[i] = mustParseFixtureBig(input.DataHash, fmt.Sprintf("inputs[%d].dataHash", i))
		w.InSeed[i] = mustParseFixtureBig(input.Seed, fmt.Sprintf("inputs[%d].seed", i))
		w.InProgramID[i] = mustParseFixtureBig(input.ProgramID, fmt.Sprintf("inputs[%d].programId", i))
		w.InLeafIndex[i] = mustParseFixtureBig(input.LeafIndex, fmt.Sprintf("inputs[%d].leafIndex", i))
		w.AccountOwnerHash[i] = mustParseFixtureBig(input.AccountOwnerHash, fmt.Sprintf("inputs[%d].accountOwnerHash", i))
		w.AccountTreeHash[i] = mustParseFixtureBig(input.AccountTreeHash, fmt.Sprintf("inputs[%d].accountTreeHash", i))
		w.AccountDiscriminator[i] = mustParseFixtureBig(input.AccountDiscriminator, fmt.Sprintf("inputs[%d].accountDiscriminator", i))
		stateRoot := mustParseFixtureBig(input.StateRoot, fmt.Sprintf("inputs[%d].stateRoot", i))
		nullifierRoot := mustParseFixtureBig(input.NullifierRoot, fmt.Sprintf("inputs[%d].nullifierRoot", i))
		if i == 0 {
			w.StateRoot = stateRoot
			w.NullifierRoot = nullifierRoot
		} else if w.StateRoot.Cmp(stateRoot) != 0 || w.NullifierRoot.Cmp(nullifierRoot) != 0 {
			return nil, fmt.Errorf("zone fixture currently requires shared state/nullifier roots")
		}
		w.StateProofs[i] = StateTreeWitness{
			Siblings:   mustParseFixtureBigSlice(input.StatePath, StateTreeHeight, fmt.Sprintf("inputs[%d].statePath", i)),
			Directions: mustCheckFixtureDirs(input.StateDirs, StateTreeHeight, fmt.Sprintf("inputs[%d].stateDirs", i)),
			Root:       stateRoot,
		}
		w.NonInclusionWs[i] = NonInclusionWitness{
			LowValue:   mustParseFixtureBig(input.NfLowValue, fmt.Sprintf("inputs[%d].nfLowValue", i)),
			NextValue:  mustParseFixtureBig(input.NfNextValue, fmt.Sprintf("inputs[%d].nfNextValue", i)),
			Siblings:   mustParseFixtureBigSlice(input.NfLowPath, IndexedTreeHeight, fmt.Sprintf("inputs[%d].nfLowPath", i)),
			Directions: mustCheckFixtureDirs(input.NfLowDirs, IndexedTreeHeight, fmt.Sprintf("inputs[%d].nfLowDirs", i)),
			Root:       nullifierRoot,
		}
	}

	w.OutOwner = make([]*big.Int, w.M)
	w.OutSpl = make([]*big.Int, w.M)
	w.OutSol = make([]*big.Int, w.M)
	w.OutBlinding = make([]*big.Int, w.M)
	w.OutDataHash = make([]*big.Int, w.M)
	w.OutOwnerIsProgram = make([]*big.Int, w.M)
	w.OutOwnerProgramIndex = make([]*big.Int, w.M)
	w.OutSeed = make([]*big.Int, w.M)
	for i, output := range spec.Outputs {
		w.OutOwner[i] = mustParseFixtureBig(output.Owner, fmt.Sprintf("outputs[%d].owner", i))
		w.OutSpl[i] = mustParseFixtureBig(output.Spl, fmt.Sprintf("outputs[%d].spl", i))
		w.OutSol[i] = mustParseFixtureBig(output.Sol, fmt.Sprintf("outputs[%d].sol", i))
		w.OutBlinding[i] = mustParseFixtureBig(output.Blinding, fmt.Sprintf("outputs[%d].blinding", i))
		w.OutDataHash[i] = mustParseFixtureBig(output.DataHash, fmt.Sprintf("outputs[%d].dataHash", i))
		w.OutOwnerIsProgram[i] = mustParseFixtureBig(output.OwnerIsProgram, fmt.Sprintf("outputs[%d].ownerIsProgram", i))
		w.OutOwnerProgramIndex[i] = mustParseFixtureBig(output.OwnerProgramIndex, fmt.Sprintf("outputs[%d].ownerProgramIndex", i))
		w.OutSeed[i] = mustParseFixtureBig(output.Seed, fmt.Sprintf("outputs[%d].seed", i))
	}

	if err := w.Recompute(); err != nil {
		return nil, err
	}
	for i, input := range spec.Inputs {
		if err := assertFixtureBig(input.ExpectedUtxoHash, w.InCommits[i], fmt.Sprintf("inputs[%d].expectedUtxoHash", i)); err != nil {
			return nil, err
		}
		if err := assertFixtureBig(input.ExpectedSpendNullifier, w.Nullifiers[i], fmt.Sprintf("inputs[%d].expectedSpendNullifier", i)); err != nil {
			return nil, err
		}
		if err := assertFixtureBig(input.ExpectedCompressedAccount, w.StateLeaves[i], fmt.Sprintf("inputs[%d].expectedCompressedAccountHash", i)); err != nil {
			return nil, err
		}
	}
	return w, nil
}

func writeFixtureOutput(t *testing.T, outputPath string, fixtures map[string]any) {
	t.Helper()
	buf, err := json.MarshalIndent(fixtures, "", "  ")
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(outputPath, buf, 0600); err != nil {
		t.Fatal(err)
	}
}

func mustParseFixtureBig(value string, field string) *big.Int {
	out, ok := new(big.Int).SetString(value, 10)
	if !ok {
		panic(fmt.Sprintf("invalid decimal %s: %q", field, value))
	}
	return out
}

func mustParseFixtureBigSlice(values []string, want int, field string) []*big.Int {
	if len(values) != want {
		panic(fmt.Sprintf("%s length mismatch: got %d want %d", field, len(values), want))
	}
	out := make([]*big.Int, len(values))
	for i, value := range values {
		out[i] = mustParseFixtureBig(value, fmt.Sprintf("%s[%d]", field, i))
	}
	return out
}

func mustCheckFixtureDirs(values []int, want int, field string) []int {
	if len(values) != want {
		panic(fmt.Sprintf("%s length mismatch: got %d want %d", field, len(values), want))
	}
	for i, value := range values {
		if value != 0 && value != 1 {
			panic(fmt.Sprintf("%s[%d] must be 0 or 1, got %d", field, i, value))
		}
	}
	return values
}

func assertFixtureBig(expected string, actual *big.Int, field string) error {
	if expected == "" {
		return nil
	}
	value := mustParseFixtureBig(expected, field)
	if value.Cmp(actual) != 0 {
		return fmt.Errorf("%s mismatch: got %s want %s", field, actual, value)
	}
	return nil
}
