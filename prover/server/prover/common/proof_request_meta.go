package common

import (
	"encoding/json"
	"fmt"
)

// ProofRequestMeta contains metadata extracted from a proof request
type ProofRequestMeta struct {
	CircuitType       CircuitType
	Version           uint32
	StateTreeHeight   uint32
	AddressTreeHeight uint32
	TreeHeight        uint32
	NumInputs         uint32
	NumAddresses      uint32
	// TreeID is the merkle tree pubkey - used for fair queuing across trees
	TreeID string
	// BatchIndex is the batch sequence number within a tree - used to process batches in order
	// Lower batch indices should be processed first to enable sequential transaction submission
	BatchIndex int64
	// MASP-specific shape. NumOutputs defaults to 0 for non-MASP circuits.
	NumOutputs uint32
}

// ParseProofRequestMeta parses a JSON input and extracts CircuitType, tree heights, and additional metrics.
func ParseProofRequestMeta(data []byte) (ProofRequestMeta, error) {
	var rawInput map[string]interface{}
	err := json.Unmarshal(data, &rawInput)
	if err != nil {
		return ProofRequestMeta{}, fmt.Errorf("failed to parse JSON: %w", err)
	}

	// Extract AddressTreeHeight
	addressTreeHeight := uint32(0)
	if height, ok := rawInput["addressTreeHeight"].(float64); ok && height > 0 {
		addressTreeHeight = uint32(height)
	}

	// Extract AddressTreeHeight
	treeHeight := uint32(0)
	if height, ok := rawInput["treeHeight"].(float64); ok && height > 0 {
		treeHeight = uint32(height)
	}

	if height, ok := rawInput["height"].(float64); ok && height > 0 && treeHeight == 0 {
		treeHeight = uint32(height)
	}
	// Extract StateTreeHeight
	stateTreeHeight := uint32(0)
	if height, ok := rawInput["stateTreeHeight"].(float64); ok && height > 0 {
		stateTreeHeight = uint32(height)
	}

	// Extract CircuitType
	circuitType, ok := rawInput["circuitType"].(string)
	if !ok || circuitType == "" {
		return ProofRequestMeta{}, fmt.Errorf("missing or invalid 'circuitType' %s", rawInput)
	}

	// MASP requests carry nInputs/nOutputs and a rootContext rather than a
	// per-request tree height, so the legacy height check does not apply.
	if !IsMaspCircuit(CircuitType(circuitType)) {
		if addressTreeHeight == 0 && stateTreeHeight == 0 && treeHeight == 0 {
			return ProofRequestMeta{}, fmt.Errorf("no 'addressTreeHeight' or stateTreeHeight'or 'treeHeight' provided")
		}
	}

	version := uint32(1)
	publicInputsHash, _ := rawInput["publicInputHash"].(string)
	if publicInputsHash != "" {
		version = 2
	}

	// Extract InclusionInputs length
	numInputs := 0
	if inclusionInputs, ok := rawInput["inputCompressedAccounts"].([]interface{}); ok {
		numInputs = len(inclusionInputs)
	}

	// Extract NonInclusionInputs length
	numAddresses := 0
	if nonInclusionInputs, ok := rawInput["newAddresses"].([]interface{}); ok {
		numAddresses = len(nonInclusionInputs)
	}

	// MASP shape: nInputs/nOutputs are top-level on MASP requests rather than
	// being inferred from a slice length. Tolerate JSON numbers and integers.
	numOutputs := 0
	if IsMaspCircuit(CircuitType(circuitType)) {
		if v, ok := rawInput["nInputs"].(float64); ok && v >= 0 {
			numInputs = int(v)
		}
		if v, ok := rawInput["nOutputs"].(float64); ok && v >= 0 {
			numOutputs = int(v)
		}
	}

	// Extract TreeID for fair queuing
	treeID := ""
	if id, ok := rawInput["treeId"].(string); ok {
		treeID = id
	}
	if treeID == "" && IsMaspCircuit(CircuitType(circuitType)) {
		if rootContext, ok := rawInput["rootContext"].(map[string]interface{}); ok {
			if id, ok := rootContext["utxoTreeId"].(string); ok {
				treeID = id
			}
		}
	}

	// Extract BatchIndex for ordering proofs within a tree
	// Default to -1 to indicate no batch index (legacy requests)
	batchIndex := int64(-1)
	if idx, ok := rawInput["batchIndex"].(float64); ok {
		batchIndex = int64(idx)
	}

	return ProofRequestMeta{
		Version:           version,
		CircuitType:       CircuitType(circuitType),
		StateTreeHeight:   stateTreeHeight,
		AddressTreeHeight: addressTreeHeight,
		NumInputs:         uint32(numInputs),
		NumAddresses:      uint32(numAddresses),
		NumOutputs:        uint32(numOutputs),
		TreeID:            treeID,
		BatchIndex:        batchIndex,
	}, nil
}
