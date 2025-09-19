package v2

import (
	"fmt"
	"light/light-prover/prover/common"
)

// SetupMerkleProofCircuit sets up V2 merkle proof circuits (inclusion, non-inclusion, or combined)
// Returns MerkleProofSystem for proof verification
func SetupMerkleProofCircuit(circuit common.CircuitType, inclusionTreeHeight uint32, inclusionNumberOfCompressedAccounts uint32, nonInclusionTreeHeight uint32, nonInclusionNumberOfCompressedAccounts uint32) (*common.MerkleProofSystem, error) {
	switch circuit {
	case common.InclusionCircuitType:
		return SetupInclusion(inclusionTreeHeight, inclusionNumberOfCompressedAccounts)
	case common.NonInclusionCircuitType:
		return SetupNonInclusion(nonInclusionTreeHeight, nonInclusionNumberOfCompressedAccounts)
	case common.CombinedCircuitType:
		return SetupCombined(inclusionTreeHeight, inclusionNumberOfCompressedAccounts, nonInclusionTreeHeight, nonInclusionNumberOfCompressedAccounts)
	default:
		return nil, fmt.Errorf("invalid circuit for V2: %s", circuit)
	}
}

// SetupBatchOperationCircuit sets up V2 batch operation circuits (append, update, or address-append)
// Returns BatchProofSystem for batch proof generation and verification
func SetupBatchOperationCircuit(circuit common.CircuitType, height uint32, batchSize uint32) (*common.BatchProofSystem, error) {
	switch circuit {
	case common.BatchAppendCircuitType:
		return SetupBatchAppend(height, batchSize)
	case common.BatchUpdateCircuitType:
		return SetupBatchUpdate(height, batchSize)
	case common.BatchAddressAppendCircuitType:
		return SetupBatchAddressAppend(height, batchSize)
	default:
		return nil, fmt.Errorf("invalid batch operation circuit: %s", circuit)
	}
}
