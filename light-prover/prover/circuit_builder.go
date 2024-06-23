package prover

import (
	"encoding/json"
	"fmt"
)

type CircuitType string

const (
	InputCompressedAccounts             = "input-compressed-accounts"
	Combined                CircuitType = "combined"
	Inclusion               CircuitType = "inclusion"
	NonInclusion            CircuitType = "non-inclusion"
)

func SetupCircuit(circuit CircuitType, inclusionTreeDepth uint32, inclusionNumberOfCompressedAccounts uint32, nonInclusionTreeDepth uint32, nonInclusionNumberOfCompressedAccounts uint32) (*ProvingSystem, error) {
	if circuit == Inclusion {
		return SetupInclusion(inclusionTreeDepth, inclusionNumberOfCompressedAccounts)
	} else if circuit == NonInclusion {
		return SetupNonInclusion(nonInclusionTreeDepth, nonInclusionNumberOfCompressedAccounts)
	} else if circuit == Combined {
		return SetupCombined(inclusionTreeDepth, inclusionNumberOfCompressedAccounts, nonInclusionTreeDepth, nonInclusionNumberOfCompressedAccounts)
	} else {
		return nil, fmt.Errorf("invalid circuit: %s", circuit)
	}
}

func ParseCircuitType(data []byte) (CircuitType, error) {
	var inputs map[string]*json.RawMessage
	err := json.Unmarshal(data, &inputs)
	if err != nil {
		return "", err
	}

	var _, hasInputCompressedAccounts = inputs["input-compressed-accounts"]
	var _, hasNewAddresses = inputs["new-addresses"]

	if hasInputCompressedAccounts && hasNewAddresses {
		return Combined, nil
	} else if hasInputCompressedAccounts {
		return Inclusion, nil
	} else if hasNewAddresses {
		return NonInclusion, nil
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
