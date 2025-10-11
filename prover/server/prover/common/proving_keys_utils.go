package common

import (
	"bytes"
	"fmt"
	"io"
	"light/light-prover/logging"
	"os"
	"path/filepath"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/constraint"
	gnarkio "github.com/consensys/gnark/io"
)

type RunMode string

const (
	Forester     RunMode = "forester"
	ForesterTest RunMode = "forester-test"
	Rpc          RunMode = "rpc"
	Full         RunMode = "full"
	FullTest     RunMode = "full-test"
	LocalRpc     RunMode = "local-rpc"
)

// Trusted setup utility functions
// Taken from: https://github.com/bnb-chain/zkbnb/blob/master/common/prove/proof_keys.go#L19

func LoadProvingKey(filepath string) (pk groth16.ProvingKey, err error) {
	logging.Logger().Info().
		Str("filepath", filepath).
		Msg("start reading proving key")

	pk = groth16.NewProvingKey(ecc.BN254)
	f, err := os.Open(filepath)
	if err != nil {
		logging.Logger().Error().
			Str("filepath", filepath).
			Err(err).
			Msg("error opening proving key file")
		return pk, fmt.Errorf("error opening proving key file: %v", err)
	}
	defer f.Close()

	fileInfo, err := f.Stat()
	if err != nil {
		logging.Logger().Error().
			Str("filepath", filepath).
			Err(err).
			Msg("error getting proving key file info")
		return pk, fmt.Errorf("error getting file info: %v", err)
	}

	logging.Logger().Info().
		Str("filepath", filepath).
		Int64("size", fileInfo.Size()).
		Msg("proving key file stats")

	n, err := pk.ReadFrom(f)
	if err != nil {
		logging.Logger().Error().
			Str("filepath", filepath).
			Int64("bytesRead", n).
			Err(err).
			Msg("error reading proving key file")
		return pk, fmt.Errorf("error reading proving key: %v", err)
	}

	logging.Logger().Info().
		Str("filepath", filepath).
		Int64("bytesRead", n).
		Msg("successfully read proving key")

	return pk, nil
}

// Taken from: https://github.com/bnb-chain/zkbnb/blob/master/common/prove/proof_keys.go#L32
func LoadVerifyingKey(filepath string) (verifyingKey groth16.VerifyingKey, err error) {
	logging.Logger().Info().Msg("start reading verifying key")
	verifyingKey = groth16.NewVerifyingKey(ecc.BN254)
	f, _ := os.Open(filepath)
	_, err = verifyingKey.ReadFrom(f)
	if err != nil {
		return verifyingKey, fmt.Errorf("read file error")
	}
	err = f.Close()
	if err != nil {
		return nil, err
	}

	return verifyingKey, nil
}

func LoadConstraintSystem(filepath string) (constraint.ConstraintSystem, error) {
	logging.Logger().Info().Str("filepath", filepath).Msg("start reading constraint system")
	cs := groth16.NewCS(ecc.BN254)
	f, err := os.Open(filepath)
	if err != nil {
		return nil, fmt.Errorf("error opening constraint system file: %v", err)
	}
	defer f.Close()

	_, err = cs.ReadFrom(f)
	if err != nil {
		return nil, fmt.Errorf("error reading constraint system: %v", err)
	}

	return cs, nil
}

func GetKeys(keysDir string, runMode RunMode, circuits []string) []string {
	var keys []string

	// Build inclusion keys
	var inclusionKeys []string
	// V2 inclusion keys (height 32) - 1 to 20
	for i := 1; i <= 20; i++ {
		inclusionKeys = append(inclusionKeys, filepath.Join(keysDir, fmt.Sprintf("v2_inclusion_32_%d.key", i)))
	}
	// V1 inclusion keys (legacy, height 26)
	for _, i := range []int{1, 2, 3, 4, 8} {
		inclusionKeys = append(inclusionKeys, filepath.Join(keysDir, fmt.Sprintf("v1_inclusion_26_%d.key", i)))
	}

	// Build non-inclusion keys
	var nonInclusionKeys []string
	// V1 non-inclusion keys (legacy, height 26)
	for i := 1; i <= 2; i++ {
		nonInclusionKeys = append(nonInclusionKeys, filepath.Join(keysDir, fmt.Sprintf("v1_non-inclusion_26_%d.key", i)))
	}
	// V2 non-inclusion keys (height 40) - 1 to 32
	for i := 1; i <= 32; i++ {
		nonInclusionKeys = append(nonInclusionKeys, filepath.Join(keysDir, fmt.Sprintf("v2_non-inclusion_40_%d.key", i)))
	}

	// Build combined keys
	var combinedKeys []string
	// V1 combined keys (legacy, heights 26/26)
	for i := 1; i <= 4; i++ {
		for j := 1; j <= 2; j++ {
			combinedKeys = append(combinedKeys, filepath.Join(keysDir, fmt.Sprintf("v1_combined_26_26_%d_%d.key", i, j)))
		}
	}
	// V2 combined keys (heights 32/40)
	for i := 1; i <= 4; i++ {
		for j := 1; j <= 4; j++ {
			combinedKeys = append(combinedKeys, filepath.Join(keysDir, fmt.Sprintf("v2_combined_32_40_%d_%d.key", i, j)))
		}
	}

	// Keys for local-rpc mode - matching the 18 keys in cli/package.json
	var localRpcKeys []string = []string{
		// V1 combined keys
		filepath.Join(keysDir, "v1_combined_26_26_1_1.key"),
		filepath.Join(keysDir, "v1_combined_26_26_1_2.key"),
		filepath.Join(keysDir, "v1_combined_26_26_2_1.key"),
		// V2 combined keys
		filepath.Join(keysDir, "v2_combined_32_40_1_1.key"),
		filepath.Join(keysDir, "v2_combined_32_40_1_2.key"),
		filepath.Join(keysDir, "v2_combined_32_40_2_1.key"),
		// V2 inclusion keys
		filepath.Join(keysDir, "v2_inclusion_32_1.key"),
		filepath.Join(keysDir, "v2_inclusion_32_2.key"),
		filepath.Join(keysDir, "v2_inclusion_32_3.key"),
		filepath.Join(keysDir, "v2_inclusion_32_4.key"),
		// V1 inclusion keys
		filepath.Join(keysDir, "v1_inclusion_26_1.key"),
		filepath.Join(keysDir, "v1_inclusion_26_2.key"),
		filepath.Join(keysDir, "v1_inclusion_26_3.key"),
		filepath.Join(keysDir, "v1_inclusion_26_4.key"),
		// V1 non-inclusion keys
		filepath.Join(keysDir, "v1_non-inclusion_26_1.key"),
		filepath.Join(keysDir, "v1_non-inclusion_26_2.key"),
		// V2 non-inclusion keys
		filepath.Join(keysDir, "v2_non-inclusion_40_1.key"),
		filepath.Join(keysDir, "v2_non-inclusion_40_2.key"),
	}

	var appendKeys []string = []string{
		filepath.Join(keysDir, "batch_append_32_500.key"),
	}

	var updateKeys []string = []string{
		filepath.Join(keysDir, "batch_update_32_500.key"),
	}

	var appendTestKeys []string = []string{
		filepath.Join(keysDir, "batch_append_32_10.key"),
	}

	var updateTestKeys []string = []string{
		filepath.Join(keysDir, "batch_update_32_10.key"),
	}

	var addressAppendKeys []string = []string{
		filepath.Join(keysDir, "batch_address-append_40_250.key"),
	}

	var addressAppendTestKeys []string = []string{
		filepath.Join(keysDir, "batch_address-append_40_10.key"),
	}

	switch runMode {
	case Forester: // inclusion + non-inclusion + append + update + address-append
		keys = append(keys, inclusionKeys...)
		keys = append(keys, nonInclusionKeys...)
		keys = append(keys, appendKeys...)
		keys = append(keys, updateKeys...)
		keys = append(keys, addressAppendKeys...)
	case ForesterTest: // inclusion + non-inclusion + combined + append-test + update-test + address-append-test
		keys = append(keys, inclusionKeys...)
		keys = append(keys, nonInclusionKeys...)
		keys = append(keys, combinedKeys...)
		keys = append(keys, appendTestKeys...)
		keys = append(keys, updateTestKeys...)
		keys = append(keys, addressAppendTestKeys...)
	case Rpc: // inclusion + non-inclusion + combined
		keys = append(keys, inclusionKeys...)
		keys = append(keys, nonInclusionKeys...)
		keys = append(keys, combinedKeys...)
	case Full: // inclusion + non-inclusion + combined + append + update + address-append
		keys = append(keys, inclusionKeys...)
		keys = append(keys, nonInclusionKeys...)
		keys = append(keys, combinedKeys...)
		keys = append(keys, updateKeys...)
		keys = append(keys, appendKeys...)
		keys = append(keys, addressAppendKeys...)
	case FullTest: // inclusion + non-inclusion + combined + append-test + update-test + address-append-test
		keys = append(keys, inclusionKeys...)
		keys = append(keys, nonInclusionKeys...)
		keys = append(keys, combinedKeys...)
		keys = append(keys, updateTestKeys...)
		keys = append(keys, appendTestKeys...)
		keys = append(keys, addressAppendTestKeys...)
	case LocalRpc:
		keys = append(keys, localRpcKeys...)
	}

	for _, circuit := range circuits {
		switch circuit {
		case "inclusion":
			keys = append(keys, inclusionKeys...)
		case "non-inclusion":
			keys = append(keys, nonInclusionKeys...)
		case "combined":
			keys = append(keys, combinedKeys...)
		case "append":
			keys = append(keys, appendKeys...)
		case "append-test":
			keys = append(keys, appendTestKeys...)
		case "update":
			keys = append(keys, updateKeys...)
		case "update-test":
			keys = append(keys, updateTestKeys...)
		case "address-append":
			keys = append(keys, addressAppendKeys...)
		case "address-append-test":
			keys = append(keys, addressAppendTestKeys...)
		}
	}
	seen := make(map[string]bool)
	var uniqueKeys []string
	for _, key := range keys {
		if !seen[key] {
			seen[key] = true
			uniqueKeys = append(uniqueKeys, key)
		}
	}

	logging.Logger().Info().
		Strs("keys", uniqueKeys).
		Msg("Loading proving system keys")

	return uniqueKeys
}

func LoadKeys(keysDirPath string, runMode RunMode, circuits []string) ([]*MerkleProofSystem, []*BatchProofSystem, error) {
	return LoadKeysWithConfig(keysDirPath, runMode, circuits, DefaultDownloadConfig())
}

func LoadKeysWithConfig(keysDirPath string, runMode RunMode, circuits []string, config *DownloadConfig) ([]*MerkleProofSystem, []*BatchProofSystem, error) {
	var pssv1 []*MerkleProofSystem
	var pssv2 []*BatchProofSystem
	keys := GetKeys(keysDirPath, runMode, circuits)

	// Ensure all required keys exist (download if necessary)
	if err := EnsureKeysExist(keys, config); err != nil {
		return nil, nil, fmt.Errorf("failed to ensure keys exist: %w", err)
	}

	for _, key := range keys {
		logging.Logger().Info().Msg("Reading proving system from file " + key + "...")
		system, err := ReadSystemFromFile(key)
		if err != nil {
			return nil, nil, err
		}
		switch s := system.(type) {
		case *MerkleProofSystem:
			pssv1 = append(pssv1, s)
			logging.Logger().Info().
				Uint32("inclusionTreeHeight", s.InclusionTreeHeight).
				Uint32("inclusionCompressedAccounts", s.InclusionNumberOfCompressedAccounts).
				Uint32("nonInclusionTreeHeight", s.NonInclusionTreeHeight).
				Uint32("nonInclusionCompressedAccounts", s.NonInclusionNumberOfCompressedAccounts).
				Msg("Read MerkleProofSystem")
		case *BatchProofSystem:
			pssv2 = append(pssv2, s)
			logging.Logger().Info().
				Uint32("treeHeight", s.TreeHeight).
				Uint32("batchSize", s.BatchSize).
				Msg("Read BatchProofSystem")
		default:
			return nil, nil, fmt.Errorf("unknown proving system type")
		}
	}
	return pssv1, pssv2, nil
}

func createFileAndWriteBytes(filePath string, data []byte) error {
	fmt.Println("Writing", len(data), "bytes to", filePath)
	file, err := os.Create(filePath)
	if err != nil {
		return err
	}
	defer func(file *os.File) {
		err := file.Close()
		if err != nil {
			return
		}
	}(file)

	_, err = io.WriteString(file, fmt.Sprintf("%d", data))
	if err != nil {
		return err
	}
	fmt.Println("Wrote", len(data), "bytes to", filePath)
	return nil
}

func WriteProvingSystem(system interface{}, path string, pathVkey string) error {
	file, err := os.Create(path)
	if err != nil {
		return err
	}
	defer file.Close()

	var written int64
	switch s := system.(type) {
	case *MerkleProofSystem:
		written, err = s.WriteTo(file)
	case *BatchProofSystem:
		written, err = s.WriteTo(file)
	default:
		return fmt.Errorf("unknown proving system type")
	}

	if err != nil {
		return err
	}

	logging.Logger().Info().Int64("bytesWritten", written).Msg("Proving system written to file")

	// Only write separate vkey file if path is provided
	if pathVkey != "" {
		var vk interface{}
		switch s := system.(type) {
		case *MerkleProofSystem:
			vk = s.VerifyingKey
		case *BatchProofSystem:
			vk = s.VerifyingKey
		}

		var buf bytes.Buffer
		_, err = vk.(gnarkio.WriterRawTo).WriteRawTo(&buf)
		if err != nil {
			return err
		}

		// Write vkey in text format for cargo xtask: [byte1 byte2 byte3 ...]
		proofBytes := buf.Bytes()
		vkeyFile, err := os.Create(pathVkey)
		if err != nil {
			return err
		}
		defer vkeyFile.Close()

		vkeyFile.WriteString("[")
		for i, b := range proofBytes {
			if i > 0 {
				vkeyFile.WriteString(" ")
			}
			fmt.Fprintf(vkeyFile, "%d", b)
		}
		vkeyFile.WriteString("]")
	}

	return nil
}
