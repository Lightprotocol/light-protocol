package prover

import (
	"bytes"
	"fmt"
	"io"
	"light/light-prover/logging"
	"os"

	gnarkio "github.com/consensys/gnark/io"
)

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
