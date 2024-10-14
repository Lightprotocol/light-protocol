package prover

import (
	"encoding/json"
	"fmt"
)

type CircuitType string

const (
	Combined     CircuitType = "combined"
	Inclusion    CircuitType = "inclusion"
	NonInclusion CircuitType = "non-inclusion"
	BatchAppend  CircuitType = "append"
	BatchUpdate  CircuitType = "update"
)

func SetupCircuitV1(circuit CircuitType, inclusionTreeHeight uint32, inclusionNumberOfCompressedAccounts uint32, nonInclusionTreeHeight uint32, nonInclusionNumberOfCompressedAccounts uint32) (*ProvingSystemV1, error) {
	switch circuit {
	case Inclusion:
		return SetupInclusion(inclusionTreeHeight, inclusionNumberOfCompressedAccounts)
	case NonInclusion:
		return SetupNonInclusion(nonInclusionTreeHeight, nonInclusionNumberOfCompressedAccounts)
	case Combined:
		return SetupCombined(inclusionTreeHeight, inclusionNumberOfCompressedAccounts, nonInclusionTreeHeight, nonInclusionNumberOfCompressedAccounts)
	default:
		return nil, fmt.Errorf("invalid circuit: %s", circuit)
	}
}

func SetupCircuitV2(circuit CircuitType, height uint32, batchSize uint32) (*ProvingSystemV2, error) {
	switch circuit {
	case BatchAppend:
		return SetupBatchAppend(height, batchSize)
	case BatchUpdate:
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
	_, hasBatchUpdateInputs := inputs["batch-update-inputs"]

	if hasInputCompressedAccounts && hasNewAddresses {
		return Combined, nil
	} else if hasInputCompressedAccounts {
		return Inclusion, nil
	} else if hasNewAddresses {
		return NonInclusion, nil
	} else if hasOldSubTreeHashChain && hasNewSubTreeHashChain && hasLeaves {
		return BatchAppend, nil
	} else if hasBatchUpdateInputs {
		return BatchUpdate, nil
	}
	return "", fmt.Errorf("unknown schema")
}

func IsCircuitEnabled(s []CircuitType, e CircuitType) bool {
	for _, a := range s {
		if a == e {
			return true
		}
	}
	return false
}
