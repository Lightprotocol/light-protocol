package masp

import (
	"errors"
	"testing"

	"light/light-prover/prover/common"
)

func TestParseMaspBaseRequest_Utxo(t *testing.T) {
	body := []byte(`{
		"circuitType": "masp-utxo",
		"nInputs": 4,
		"nOutputs": 2,
		"publicInputsHash": "0x01",
		"rootContext": {
			"utxoTreeId": "tree1",
			"utxoRootIndex": 7,
			"nullifierRootIndex": 9
		}
	}`)
	req, err := ParseMaspBaseRequest(body)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if req.CircuitType != common.MaspUtxoCircuitType {
		t.Fatalf("want masp-utxo, got %s", req.CircuitType)
	}
	if req.NInputs != 4 || req.NOutputs != 2 {
		t.Fatalf("want 4/2, got %d/%d", req.NInputs, req.NOutputs)
	}
	if req.RootContext.UtxoRootIndex != 7 {
		t.Fatalf("rootContext utxoRootIndex did not survive decoding")
	}
}

func TestParseMaspBaseRequest_RejectsNonMasp(t *testing.T) {
	body := []byte(`{"circuitType": "inclusion", "nInputs": 1, "nOutputs": 1}`)
	if _, err := ParseMaspBaseRequest(body); err == nil {
		t.Fatalf("expected rejection of non-MASP circuit type")
	}
}

func TestParseMaspBaseRequest_RequiresOutputsForUtxo(t *testing.T) {
	body := []byte(`{"circuitType": "masp-utxo", "nInputs": 1}`)
	if _, err := ParseMaspBaseRequest(body); err == nil {
		t.Fatalf("masp-utxo without nOutputs must fail")
	}
}

func TestKeyManager_GetMaspSystemRequiresBuilder(t *testing.T) {
	km := common.NewLazyKeyManager("/tmp/", nil)
	_, err := km.GetMaspSystem(common.MaspUtxoCircuitType, 1, 1)
	if err == nil {
		t.Fatalf("expected error when no builder is registered")
	}
}

func TestKeyManager_GetMaspSystemRejectsNonMasp(t *testing.T) {
	km := common.NewLazyKeyManager("/tmp/", nil)
	_, err := km.GetMaspSystem(common.InclusionCircuitType, 1, 1)
	if err == nil || !contains(err.Error(), "not a MASP circuit type") {
		t.Fatalf("want non-MASP rejection, got %v", err)
	}
}

func contains(s, sub string) bool {
	return len(sub) == 0 || (len(s) >= len(sub) && (indexOf(s, sub) >= 0))
}

func indexOf(s, sub string) int {
	for i := 0; i+len(sub) <= len(s); i++ {
		if s[i:i+len(sub)] == sub {
			return i
		}
	}
	return -1
}

var _ = errors.New
