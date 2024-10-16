package prover

import (
	"encoding/json"
	"fmt"
)

type CircuitType string

const (
	CombinedCircuitType     CircuitType = "combined"
	InclusionCircuitType    CircuitType = "inclusion"
	NonInclusionCircuitType CircuitType = "non-inclusion"
	BatchAppendCircuitType  CircuitType = "append"
	BatchAppend2CircuitType CircuitType = "append2"
	BatchUpdateCircuitType  CircuitType = "update"
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
	case BatchAppendCircuitType:
		return SetupBatchAppend(height, batchSize)
	case BatchAppend2CircuitType:
		return SetupBatchAppend2(height, batchSize)
	case BatchUpdateCircuitType:
		return SetupBatchUpdate(height, batchSize)
	default:
		return nil, fmt.Errorf("invalid circuit: %s", circuit)
	}
}

func ParseCircuitType(data []byte) (CircuitType, error) {
	var inputs map[string]*json.RawMessage
	err := json.Unmarshal(data, &inputs)
	if err != nil {
		return "", err
	}

	_, hasInputCompressedAccounts := inputs["input-compressed-accounts"]
	_, hasNewAddresses := inputs["new-addresses"]
	_, hasOldSubTreeHashChain := inputs["oldSubTreeHashChain"]
	_, hasNewSubTreeHashChain := inputs["newSubTreeHashChain"]
	_, hasLeaves := inputs["leaves"]
	_, hasNewMerkleProofs := inputs["newMerkleProofs"]
	_, hasOldLeaves := inputs["oldLeaves"]

	if hasInputCompressedAccounts && hasNewAddresses {
		return CombinedCircuitType, nil
	} else if hasInputCompressedAccounts {
		return InclusionCircuitType, nil
	} else if hasNewAddresses {
		return NonInclusionCircuitType, nil
	} else if hasOldSubTreeHashChain && hasNewSubTreeHashChain && hasLeaves {
		return BatchAppendCircuitType, nil
	} else if hasNewMerkleProofs {
		return BatchUpdateCircuitType, nil
	} else if hasOldLeaves {
		return BatchAppend2CircuitType, nil
	}
	return "", fmt.Errorf("unknown schema")
}
