package prover

import (
	"light/light-prover/prover/common"
	"light/light-prover/prover/v1"
	"light/light-prover/prover/v2"
)

// SetupMerkleProofCircuit sets up merkle proof circuits (inclusion, non-inclusion, or combined)
// Returns MerkleProofSystem for proof verification
// This is a wrapper function that delegates to version-specific implementations
func SetupMerkleProofCircuit(circuit common.CircuitType, inclusionTreeHeight uint32, inclusionNumberOfCompressedAccounts uint32, nonInclusionTreeHeight uint32, nonInclusionNumberOfCompressedAccounts uint32, useV1 bool) (*common.MerkleProofSystem, error) {
	if useV1 {
		return v1.SetupMerkleProofCircuit(circuit, inclusionTreeHeight, inclusionNumberOfCompressedAccounts, nonInclusionTreeHeight, nonInclusionNumberOfCompressedAccounts)
	}
	return v2.SetupMerkleProofCircuit(circuit, inclusionTreeHeight, inclusionNumberOfCompressedAccounts, nonInclusionTreeHeight, nonInclusionNumberOfCompressedAccounts)
}

// SetupBatchOperationCircuit sets up batch operation circuits (append, update, or address-append)
// Returns BatchProofSystem for batch proof generation and verification
// This is a wrapper function that delegates to V2 implementation (batch operations are V2 only)
func SetupBatchOperationCircuit(circuit common.CircuitType, height uint32, batchSize uint32) (*common.BatchProofSystem, error) {
	return v2.SetupBatchOperationCircuit(circuit, height, batchSize)
}

// Re-export types and functions from common for backward compatibility
type ProofRequestMeta = common.ProofRequestMeta

// ParseProofRequestMeta re-exports the common function for backward compatibility
func ParseProofRequestMeta(data []byte) (ProofRequestMeta, error) {
	return common.ParseProofRequestMeta(data)
}
