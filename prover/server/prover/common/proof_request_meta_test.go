package common

import (
	"testing"
)

func TestParseProofRequestMeta_MaspUtxo(t *testing.T) {
	body := []byte(`{
		"circuitType": "masp-utxo",
		"nInputs": 4,
		"nOutputs": 2,
		"publicInputsHash": "0x01",
		"rootContext": {"utxoRoot": "0x00"}
	}`)
	meta, err := ParseProofRequestMeta(body)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if meta.CircuitType != MaspUtxoCircuitType {
		t.Fatalf("want masp-utxo, got %s", meta.CircuitType)
	}
	if meta.NumInputs != 4 || meta.NumOutputs != 2 {
		t.Fatalf("want 4/2 inputs/outputs, got %d/%d", meta.NumInputs, meta.NumOutputs)
	}
}

func TestParseProofRequestMeta_MaspTree(t *testing.T) {
	body := []byte(`{
		"circuitType": "masp-tree",
		"nInputs": 2,
		"publicInputsHash": "0xff"
	}`)
	meta, err := ParseProofRequestMeta(body)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !IsMaspCircuit(meta.CircuitType) {
		t.Fatalf("expected MASP circuit type, got %s", meta.CircuitType)
	}
	if meta.NumInputs != 2 {
		t.Fatalf("want 2 inputs, got %d", meta.NumInputs)
	}
}

func TestParseProofRequestMeta_NonMaspStillRequiresHeight(t *testing.T) {
	body := []byte(`{"circuitType": "inclusion"}`)
	_, err := ParseProofRequestMeta(body)
	if err == nil {
		t.Fatalf("expected an error when no tree height is provided for a non-MASP circuit")
	}
}

func TestIsMaspCircuit(t *testing.T) {
	for _, c := range []CircuitType{MaspUtxoCircuitType, MaspTreeCircuitType, MaspBundleCircuitType} {
		if !IsMaspCircuit(c) {
			t.Errorf("IsMaspCircuit(%s) = false", c)
		}
	}
	for _, c := range []CircuitType{InclusionCircuitType, BatchAppendCircuitType, ""} {
		if IsMaspCircuit(c) {
			t.Errorf("IsMaspCircuit(%s) = true; want false", c)
		}
	}
}
