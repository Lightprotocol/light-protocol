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
	BatchAppendWithSubtreesCircuitType CircuitType = "append-with-subtrees"
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
	case BatchAppendWithSubtreesCircuitType:
		return SetupBatchAppend(height, batchSize)
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
	_, hasOldLowElements := inputs["LowElementValues"]

	if hasInputCompressedAccounts && hasNewAddresses {
		return CombinedCircuitType, nil
	} else if hasInputCompressedAccounts {
		return InclusionCircuitType, nil
	} else if hasNewAddresses {
		return NonInclusionCircuitType, nil
	} else if hasOldSubTreeHashChain && hasNewSubTreeHashChain && hasLeaves {
		return BatchAppendWithSubtreesCircuitType, nil
	} else if hasNewMerkleProofs {
		return BatchUpdateCircuitType, nil
	} else if hasOldLeaves {
		return BatchAppendWithProofsCircuitType, nil
	} else if hasOldLowElements {
		return BatchAddressAppendCircuitType, nil
	}
	return "", fmt.Errorf("unknown schema")
}
