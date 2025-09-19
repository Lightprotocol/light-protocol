package prover

import (
	"light/light-prover/prover/common"
)

// LoadKeys loads proving system keys from the specified directory
// Re-exports the common function for backward compatibility
func LoadKeys(keysDirPath string, runMode common.RunMode, circuits []string) ([]*common.MerkleProofSystem, []*common.BatchProofSystem, error) {
	return common.LoadKeys(keysDirPath, runMode, circuits)
}
