package v1

import (
	"fmt"
	"light/light-prover/prover/common"
)

// SetupMerkleProofCircuit sets up V1 merkle proof circuits (inclusion, non-inclusion, or combined)
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
		return nil, fmt.Errorf("invalid circuit for V1: %s", circuit)
	}
}
