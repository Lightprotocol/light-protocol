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

func SetupCircuit(circuit CircuitType, inclusionTreeDepth uint32, inclusionNumberOfCompressedAccounts uint32, nonInclusionTreeDepth uint32, nonInclusionNumberOfCompressedAccounts uint32, keyFilePath string) (*ProvingSystem, error) {
	switch circuit {
	case Inclusion:
		return SetupInclusion(inclusionTreeDepth, inclusionNumberOfCompressedAccounts, keyFilePath)
	case NonInclusion:
		return SetupNonInclusion(nonInclusionTreeDepth, nonInclusionNumberOfCompressedAccounts, keyFilePath)
	case Combined:
		return SetupCombined(inclusionTreeDepth, inclusionNumberOfCompressedAccounts, nonInclusionTreeDepth, nonInclusionNumberOfCompressedAccounts, keyFilePath)
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

func GenerateKeyFilePath(baseDir string, circuit CircuitType, inclusionTreeDepth, inclusionCompressedAccounts, nonInclusionTreeDepth, nonInclusionCompressedAccounts uint32) string {
	switch circuit {
	case Inclusion:
		return fmt.Sprintf("%s/inclusion_%d_%d", baseDir, inclusionTreeDepth, inclusionCompressedAccounts)
	case NonInclusion:
		return fmt.Sprintf("%s/non-inclusion_%d_%d", baseDir, nonInclusionTreeDepth, nonInclusionCompressedAccounts)
	case Combined:
		return fmt.Sprintf("%s/combined_%d_%d_%d_%d", baseDir, inclusionTreeDepth, inclusionCompressedAccounts, nonInclusionTreeDepth, nonInclusionCompressedAccounts)
	default:
		return ""
	}
}
