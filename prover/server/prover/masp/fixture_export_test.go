package masp

import (
	"encoding/json"
	"os"
	"testing"

	"light/light-prover/prover/common"
)

func TestExportLocalDevProofRequestFixtures(t *testing.T) {
	outputPath := os.Getenv("MASP_FIXTURE_OUT")
	if outputPath == "" {
		t.Skip("set MASP_FIXTURE_OUT to export local/dev MASP request fixtures")
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

	buf, err := json.MarshalIndent(fixtures, "", "  ")
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(outputPath, buf, 0600); err != nil {
		t.Fatal(err)
	}
}
