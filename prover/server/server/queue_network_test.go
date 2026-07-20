package server

import (
	"encoding/json"
	"testing"
)

func TestNormalizeNetwork(t *testing.T) {
	for input, expected := range map[string]string{
		"mainnet":   MainnetNetwork,
		" MAINNET ": MainnetNetwork,
		"devnet":    DevnetNetwork,
	} {
		actual, err := normalizeNetwork(input)
		if err != nil {
			t.Fatalf("normalizeNetwork(%q): %v", input, err)
		}
		if actual != expected {
			t.Fatalf("normalizeNetwork(%q) = %q, want %q", input, actual, expected)
		}
	}

	for _, input := range []string{"", "default", "testnet", "mainnet-beta"} {
		if _, err := normalizeNetwork(input); err == nil {
			t.Fatalf("normalizeNetwork(%q) unexpectedly succeeded", input)
		}
	}
}

func TestNetworkQueueNames(t *testing.T) {
	request := NetworkQueueName("zk_address_append_queue", MainnetNetwork)
	if request != "zk_address_append_mainnet_queue" {
		t.Fatalf("unexpected request queue: %s", request)
	}
	processing := ProcessingQueueName(request)
	if processing != "zk_address_append_mainnet_processing_queue" {
		t.Fatalf("unexpected processing queue: %s", processing)
	}
	if original := getOriginalQueueFromProcessing(processing); original != request {
		t.Fatalf("processing queue maps to %s, want %s", original, request)
	}
}

func TestNetworkInputHashIsolation(t *testing.T) {
	payload := json.RawMessage(`{"circuitType":"batchAppend"}`)
	mainnet := ComputeNetworkInputHash(MainnetNetwork, payload)
	devnet := ComputeNetworkInputHash(DevnetNetwork, payload)
	legacy := ComputeNetworkInputHash("", payload)

	if mainnet == devnet || mainnet == legacy || devnet == legacy {
		t.Fatal("identical payloads must have distinct cache keys across networks")
	}
	if legacy != ComputeInputHash(payload) {
		t.Fatal("legacy input hash changed")
	}
}

func TestQueueLimits(t *testing.T) {
	t.Setenv("PROVER_MAX_PENDING_MAINNET", "21")
	t.Setenv("PROVER_MAX_PENDING_DEVNET", "3")

	if got := queueLimit("zk_append_mainnet_queue"); got != 21 {
		t.Fatalf("mainnet queue limit = %d, want 21", got)
	}
	if got := queueLimit("zk_append_devnet_queue"); got != 3 {
		t.Fatalf("devnet queue limit = %d, want 3", got)
	}
}
