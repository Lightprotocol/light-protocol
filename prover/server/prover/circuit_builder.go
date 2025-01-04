package prover

import (
	"encoding/json"
	"fmt"
)

type CircuitType string

const (
	CombinedCircuitType                CircuitType = "combined"
	InclusionCircuitType               CircuitType = "inclusion"
	NonInclusionCircuitType            CircuitType = "non-inclusion"
	BatchAppendWithProofsCircuitType   CircuitType = "append-with-proofs"
	BatchUpdateCircuitType             CircuitType = "update"
	BatchAddressAppendCircuitType      CircuitType = "address-append"
)

func SetupCircuitV1(circuit CircuitType, inclusionTreeHeight uint32, inclusionNumberOfCompressedAccounts uint32, nonInclusionTreeHeight uint32, nonInclusionNumberOfCompressedAccounts uint32) (*ProvingSystemV1, error) {
	switch circuit {
	case InclusionCircuitType:
		return SetupInclusion(inclusionTreeHeight, inclusionNumberOfCompressedAccounts)
	case NonInclusionCircuitType:
		return SetupNonInclusion(nonInclusionTreeHeight, nonInclusionNumberOfCompressedAccounts)
	case CombinedCircuitType:
		return SetupCombined(inclusionTreeHeight, inclusionNumberOfCompressedAccounts, nonInclusionTreeHeight, nonInclusionNumberOfCompressedAccounts)
	default:
		return nil, fmt.Errorf("invalid circuit: %s", circuit)
	}
}

func SetupCircuitV2(circuit CircuitType, height uint32, batchSize uint32) (*ProvingSystemV2, error) {
	switch circuit {
	case BatchAppendWithProofsCircuitType:
		return SetupBatchAppendWithProofs(height, batchSize)
	case BatchUpdateCircuitType:
		return SetupBatchUpdate(height, batchSize)
	case BatchAddressAppendCircuitType:
		return SetupBatchAddressAppend(height, batchSize)
	default:
		return nil, fmt.Errorf("invalid circuit: %s", circuit)
	}
}

type ProofRequestMeta struct {
	CircuitType       CircuitType
	Version           uint32
	StateTreeHeight   uint32
	AddressTreeHeight uint32
	TreeHeight        uint32
	NumInputs         uint32
	NumAddresses      uint32
}

// ParseParameters parses a JSON input and extracts CircuitType, tree heights, and additional metrics.
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

	if addressTreeHeight == 0 && stateTreeHeight == 0 && treeHeight == 0 {
		return ProofRequestMeta{}, fmt.Errorf("no 'addressTreeHeight' or stateTreeHeight'or 'treeHeight' provided")
	}

	version := uint32(0)
	publicInputsHash, _ := rawInput["publicInputHash"].(string)
	if publicInputsHash != "" {
		version = 1
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

	return ProofRequestMeta{
		Version:           version,
		CircuitType:       CircuitType(circuitType),
		StateTreeHeight:   stateTreeHeight,
		AddressTreeHeight: addressTreeHeight,
		NumInputs:         uint32(numInputs),
		NumAddresses:      uint32(numAddresses),
	}, nil
}
