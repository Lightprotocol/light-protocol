package prover

import (
	"bytes"
	"fmt"
	"io"
	"light/light-prover/logging"
	"os"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
	gnarkio "github.com/consensys/gnark/io"
)

type RunMode string

const (
	Forester     RunMode = "forester"
	ForesterTest RunMode = "forester-test"
	Rpc          RunMode = "rpc"
	Full         RunMode = "full"
	FullTest     RunMode = "full-test"
)

// Trusted setup utility functions
// Taken from: https://github.com/bnb-chain/zkbnb/blob/master/common/prove/proof_keys.go#L19
func LoadProvingKey(filepath string) (pk groth16.ProvingKey, err error) {
	logging.Logger().Info().Msg("start reading proving key")
	pk = groth16.NewProvingKey(ecc.BN254)
	f, _ := os.Open(filepath)
	_, err = pk.ReadFrom(f)
	if err != nil {
		return pk, fmt.Errorf("read file error")
	}
	err = f.Close()
	if err != nil {
		return nil, err
	}
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

func GetKeys(keysDir string, runMode RunMode, circuits []string) []string {
	var keys []string

	var inclusionKeys []string = []string{
		keysDir + "inclusion_26_1.key",
		keysDir + "inclusion_26_2.key",
		keysDir + "inclusion_26_3.key",
		keysDir + "inclusion_26_4.key",
		keysDir + "inclusion_26_8.key",
	}

	var nonInclusionKeys []string = []string{
		keysDir + "non-inclusion_26_1.key",
		keysDir + "non-inclusion_26_2.key",
	}

	var combinedKeys []string = []string{
		keysDir + "combined_26_1_1.key",
		keysDir + "combined_26_1_2.key",
		keysDir + "combined_26_2_1.key",
		keysDir + "combined_26_2_2.key",
		keysDir + "combined_26_3_1.key",
		keysDir + "combined_26_3_2.key",
		keysDir + "combined_26_4_1.key",
		keysDir + "combined_26_4_2.key",
	}

	var appendKeys []string = []string{
		keysDir + "append_26_1.key",
		keysDir + "append_26_10.key",
		keysDir + "append_26_100.key",
		keysDir + "append_26_500.key",
		keysDir + "append_26_1000.key",
	}
	var updateKeys []string = []string{
		keysDir + "update_26_1.key",
		keysDir + "update_26_10.key",
		keysDir + "update_26_100.key",
		keysDir + "update_26_500.key",
		keysDir + "update_26_1000.key",
	}

	var appendTestKeys []string = []string{
		keysDir + "append_26_10.key",
	}
	var updateTestKeys []string = []string{
		keysDir + "update_26_10.key",
	}
	var append2TestKeys []string = []string{
		keysDir + "append2_26_10.key",
	}

	switch runMode {
	case Forester: // inclusion + non-inclusion
		keys = append(keys, inclusionKeys...)
		keys = append(keys, nonInclusionKeys...)
	case ForesterTest: // append-test + update-test
		keys = append(keys, append2TestKeys...)
		keys = append(keys, updateTestKeys...)
	case Rpc: // inclusion + non-inclusion + combined
		keys = append(keys, inclusionKeys...)
		keys = append(keys, nonInclusionKeys...)
		keys = append(keys, combinedKeys...)
	case Full: // inclusion + non-inclusion + combined + append + update
		keys = append(keys, inclusionKeys...)
		keys = append(keys, nonInclusionKeys...)
		keys = append(keys, combinedKeys...)
		keys = append(keys, appendKeys...)
		keys = append(keys, updateKeys...)
	case FullTest: // inclusion + non-inclusion + combined + append-test + update-test
		keys = append(keys, inclusionKeys...)
		keys = append(keys, nonInclusionKeys...)
		keys = append(keys, combinedKeys...)
		keys = append(keys, appendTestKeys...)
		keys = append(keys, updateTestKeys...)
		keys = append(keys, append2TestKeys...)
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
		case "update":
			keys = append(keys, updateKeys...)
		case "append-test":
			keys = append(keys, appendTestKeys...)
		case "update-test":
			keys = append(keys, updateTestKeys...)
		case "append2-test":
			keys = append(keys, append2TestKeys...)
		}
	}

	return keys
}

func LoadKeys(keysDirPath string, runMode RunMode, circuits []string) ([]*ProvingSystemV1, []*ProvingSystemV2, error) {
	var pssv1 []*ProvingSystemV1
	var pssv2 []*ProvingSystemV2
	keys := GetKeys(keysDirPath, runMode, circuits)

	for _, key := range keys {
		logging.Logger().Info().Msg("Reading proving system from file " + key + "...")
		system, err := ReadSystemFromFile(key)
		if err != nil {
			return nil, nil, err
		}
		switch s := system.(type) {
		case *ProvingSystemV1:
			pssv1 = append(pssv1, s)
			logging.Logger().Info().
				Uint32("inclusionTreeHeight", s.InclusionTreeHeight).
				Uint32("inclusionCompressedAccounts", s.InclusionNumberOfCompressedAccounts).
				Uint32("nonInclusionTreeHeight", s.NonInclusionTreeHeight).
				Uint32("nonInclusionCompressedAccounts", s.NonInclusionNumberOfCompressedAccounts).
				Msg("Read ProvingSystem")
		case *ProvingSystemV2:
			pssv2 = append(pssv2, s)
			logging.Logger().Info().
				Uint32("treeHeight", s.TreeHeight).
				Uint32("batchSize", s.BatchSize).
				Msg("Read BatchAppendProvingSystem")
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
		return err // Return the error to the caller
	}
	defer func(file *os.File) {
		err := file.Close()
		if err != nil {
			return
		}
	}(file)

	_, err = io.WriteString(file, fmt.Sprintf("%d", data))
	if err != nil {
		return err // Return any error that occurs during writing
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
	case *ProvingSystemV1:
		written, err = s.WriteTo(file)
	case *ProvingSystemV2:
		written, err = s.WriteTo(file)
	default:
		return fmt.Errorf("unknown proving system type")
	}

	if err != nil {
		return err
	}

	logging.Logger().Info().Int64("bytesWritten", written).Msg("Proving system written to file")

	// Write verification key
	var vk interface{}
	switch s := system.(type) {
	case *ProvingSystemV1:
		vk = s.VerifyingKey
	case *ProvingSystemV2:
		vk = s.VerifyingKey
	}

	var buf bytes.Buffer
	_, err = vk.(gnarkio.WriterRawTo).WriteRawTo(&buf)
	if err != nil {
		return err
	}

	proofBytes := buf.Bytes()
	err = createFileAndWriteBytes(pathVkey, proofBytes)
	if err != nil {
		return err
	}

	return nil
}
