package main

import (
	"bytes"
	_ "embed"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"light/light-prover/logging"
	"light/light-prover/prover"
	"light/light-prover/server"
	"math/big"
	"os"
	"os/signal"
	"path/filepath"
	"strings"

	"github.com/consensys/gnark/constraint"
	gnarkio "github.com/consensys/gnark/io"
	gnarkLogger "github.com/consensys/gnark/logger"
	"github.com/urfave/cli/v2"
)

func main() {
	runCli()
}

func runCli() {
	gnarkLogger.Set(*logging.Logger())
	app := cli.App{
		EnableBashCompletion: true,
		Commands: []*cli.Command{
			{
				Name: "setup",
				Flags: []cli.Flag{
					&cli.StringFlag{Name: "circuit", Usage: "Type of circuit (\"inclusion\" / \"non-inclusion\" / \"combined\" / \"append\" )", Required: true},
					&cli.StringFlag{Name: "output", Usage: "Output file", Required: true},
					&cli.StringFlag{Name: "output-vkey", Usage: "Output file", Required: true},
					&cli.UintFlag{Name: "inclusion-tree-height", Usage: "[Inclusion]: Merkle tree height", Required: false},
					&cli.UintFlag{Name: "inclusion-compressed-accounts", Usage: "[Inclusion]: Number of compressed accounts", Required: false},
					&cli.UintFlag{Name: "non-inclusion-tree-height", Usage: "[Non-inclusion]: merkle tree height", Required: false},
					&cli.UintFlag{Name: "non-inclusion-compressed-accounts", Usage: "[Non-inclusion]: number of compressed accounts", Required: false},
					&cli.UintFlag{Name: "append-tree-height", Usage: "[Batch append]: tree height", Required: false},
					&cli.UintFlag{Name: "append-batch-size", Usage: "[Batch append]: barch size", Required: false},
					&cli.UintFlag{Name: "update-tree-height", Usage: "[Batch update]: tree height", Required: false},
					&cli.UintFlag{Name: "update-batch-size", Usage: "[Batch update]: batch size", Required: false},
				},
				Action: func(context *cli.Context) error {
					circuit := prover.CircuitType(context.String("circuit"))
					if circuit != prover.Inclusion && circuit != prover.NonInclusion && circuit != prover.Combined && circuit != prover.BatchAppend && circuit != prover.BatchUpdate {
						return fmt.Errorf("invalid circuit type %s", circuit)
					}

					path := context.String("output")
					pathVkey := context.String("output-vkey")
					inclusionTreeHeight := uint32(context.Uint("inclusion-tree-height"))
					inclusionNumberOfCompressedAccounts := uint32(context.Uint("inclusion-compressed-accounts"))
					nonInclusionTreeHeight := uint32(context.Uint("non-inclusion-tree-height"))
					nonInclusionNumberOfCompressedAccounts := uint32(context.Uint("non-inclusion-compressed-accounts"))
					batchAppendTreeHeight := uint32(context.Uint("append-tree-height"))
					batchAppendBatchSize := uint32(context.Uint("append-batch-size"))
					batchUpdateTreeHeight := uint32(context.Uint("update-tree-height"))
					batchUpdateBatchSize := uint32(context.Uint("update-batch-size"))

					if (inclusionTreeHeight == 0 || inclusionNumberOfCompressedAccounts == 0) && circuit == prover.Inclusion {
						return fmt.Errorf("inclusion tree height and number of compressed accounts must be provided")
					}

					if (nonInclusionTreeHeight == 0 || nonInclusionNumberOfCompressedAccounts == 0) && circuit == prover.NonInclusion {
						return fmt.Errorf("non-inclusion tree height and number of compressed accounts must be provided")
					}

					if circuit == prover.Combined {
						if inclusionTreeHeight == 0 || inclusionNumberOfCompressedAccounts == 0 {
							return fmt.Errorf("inclusion tree height and number of compressed accounts must be provided")
						}
						if nonInclusionTreeHeight == 0 || nonInclusionNumberOfCompressedAccounts == 0 {
							return fmt.Errorf("non-inclusion tree height and number of compressed accounts must be provided")
						}
					}

					if (batchAppendTreeHeight == 0 || batchAppendBatchSize == 0) && circuit == prover.BatchAppend {
						return fmt.Errorf("[Batch append]: tree height and batch size must be provided")
					}

					if (batchUpdateTreeHeight == 0 || batchUpdateBatchSize == 0) && circuit == prover.BatchUpdate {
						return fmt.Errorf("[Batch update]: tree height and batch size must be provided")
					}
					logging.Logger().Info().Msg("Running setup")
					var err error
					if circuit == prover.BatchAppend {
						var system *prover.ProvingSystemV2
						system, err = prover.SetupCircuitV2(prover.BatchAppend, batchAppendTreeHeight, batchAppendBatchSize)
						if err != nil {
							return err
						}
						err = writeProvingSystem(system, path, pathVkey)
					} else if circuit == prover.BatchUpdate {
						var system *prover.ProvingSystemV2
						system, err = prover.SetupCircuitV2(prover.BatchUpdate, batchUpdateTreeHeight, batchUpdateBatchSize)
						if err != nil {
							return err
						}
						err = writeProvingSystem(system, path, pathVkey)
					} else {
						var system *prover.ProvingSystemV1
						system, err = prover.SetupCircuitV1(circuit, inclusionTreeHeight, inclusionNumberOfCompressedAccounts, nonInclusionTreeHeight, nonInclusionNumberOfCompressedAccounts)
						if err != nil {
							return err
						}
						err = writeProvingSystem(system, path, pathVkey)
					}

					if err != nil {
						return err
					}

					logging.Logger().Info().Msg("Setup completed successfully")
					return nil
				},
			},
			{
				Name: "r1cs",
				Flags: []cli.Flag{
					&cli.StringFlag{Name: "output", Usage: "Output file", Required: true},
					&cli.StringFlag{Name: "circuit", Usage: "Type of circuit (\"inclusion\" / \"non-inclusion\" / \"combined\" / \"append\")", Required: true},
					&cli.UintFlag{Name: "inclusion-tree-height", Usage: "[Inclusion]: merkle tree height", Required: false},
					&cli.UintFlag{Name: "inclusion-compressed-accounts", Usage: "[Inclusion]: number of compressed accounts", Required: false},
					&cli.UintFlag{Name: "non-inclusion-tree-height", Usage: "[Non-inclusion]: merkle tree height", Required: false},
					&cli.UintFlag{Name: "non-inclusion-compressed-accounts", Usage: "[Non-inclusion]: number of compressed accounts", Required: false},
					&cli.UintFlag{Name: "append-tree-height", Usage: "[Batch append]: merkle tree height", Required: false},
					&cli.UintFlag{Name: "append-batch-size", Usage: "[Batch append]: batch size", Required: false},
					&cli.UintFlag{Name: "update-tree-height", Usage: "[Batch update]: merkle tree height", Required: false},
					&cli.UintFlag{Name: "update-batch-size", Usage: "[Batch update]: batch size", Required: false},
				},
				Action: func(context *cli.Context) error {
					circuit := prover.CircuitType(context.String("circuit"))
					if circuit != prover.Inclusion && circuit != prover.NonInclusion && circuit != prover.Combined && circuit != prover.BatchAppend {
						return fmt.Errorf("invalid circuit type %s", circuit)
					}

					path := context.String("output")
					inclusionTreeHeight := uint32(context.Uint("inclusion-tree-height"))
					inclusionNumberOfCompressedAccounts := uint32(context.Uint("inclusion-compressed-accounts"))
					nonInclusionTreeHeight := uint32(context.Uint("non-inclusion-tree-height"))
					nonInclusionNumberOfCompressedAccounts := uint32(context.Uint("non-inclusion-compressed-accounts"))
					batchAppendTreeHeight := uint32(context.Uint("append-tree-height"))
					batchAppendBatchSize := uint32(context.Uint("append-batch-size"))
					batchUpdateTreeHeight := uint32(context.Uint("update-tree-height"))
					batchUpdateBatchSize := uint32(context.Uint("update-batch-size"))

					if (inclusionTreeHeight == 0 || inclusionNumberOfCompressedAccounts == 0) && circuit == "inclusion" {
						return fmt.Errorf("[Inclusion]: tree height and number of compressed accounts must be provided")
					}

					if (nonInclusionTreeHeight == 0 || nonInclusionNumberOfCompressedAccounts == 0) && circuit == "non-inclusion" {
						return fmt.Errorf("[Non-inclusion]: tree height and number of compressed accounts must be provided")
					}

					if circuit == "combined" {
						if inclusionTreeHeight == 0 || inclusionNumberOfCompressedAccounts == 0 {
							return fmt.Errorf("[Combined]: tree height and number of compressed accounts must be provided")
						}
						if nonInclusionTreeHeight == 0 || nonInclusionNumberOfCompressedAccounts == 0 {
							return fmt.Errorf("[Combined]: tree height and number of compressed accounts must be provided")
						}
					}

					if (batchAppendTreeHeight == 0 || batchAppendBatchSize == 0) && circuit == prover.BatchAppend {
						return fmt.Errorf("[Batch append]: tree height and batch size must be provided")
					}

					if (batchUpdateTreeHeight == 0 || batchUpdateBatchSize == 0) && circuit == prover.BatchUpdate {
						return fmt.Errorf("[Batch update]: tree height and batch size must be provided")
					}

					logging.Logger().Info().Msg("Building R1CS")

					var cs constraint.ConstraintSystem
					var err error

					if circuit == prover.Inclusion {
						cs, err = prover.R1CSInclusion(inclusionTreeHeight, inclusionNumberOfCompressedAccounts)
					} else if circuit == prover.NonInclusion {
						cs, err = prover.R1CSNonInclusion(nonInclusionTreeHeight, nonInclusionNumberOfCompressedAccounts)
					} else if circuit == prover.Combined {
						cs, err = prover.R1CSCombined(inclusionTreeHeight, inclusionNumberOfCompressedAccounts, nonInclusionTreeHeight, nonInclusionNumberOfCompressedAccounts)
					} else if circuit == prover.BatchAppend {
						cs, err = prover.R1CSBatchAppend(batchAppendTreeHeight, batchAppendBatchSize)
					} else if circuit == prover.BatchUpdate {
						cs, err = prover.R1CSBatchUpdate(batchUpdateTreeHeight, batchUpdateBatchSize)
					} else {
						return fmt.Errorf("invalid circuit type %s", circuit)
					}

					if err != nil {
						return err
					}
					file, err := os.Create(path)
					defer func(file *os.File) {
						err := file.Close()
						if err != nil {
							logging.Logger().Error().Err(err).Msg("error closing file")
						}
					}(file)
					if err != nil {
						return err
					}
					written, err := cs.WriteTo(file)
					if err != nil {
						return err
					}
					logging.Logger().Info().Int64("bytesWritten", written).Msg("R1CS written to file")
					return nil
				},
			},
			{
				Name: "import-setup",
				Flags: []cli.Flag{
					&cli.StringFlag{Name: "circuit", Usage: "Type of circuit (\"inclusion\" / \"non-inclusion\" / \"combined\")", Required: true},
					&cli.StringFlag{Name: "output", Usage: "Output file", Required: true},
					&cli.StringFlag{Name: "pk", Usage: "Proving key", Required: true},
					&cli.StringFlag{Name: "vk", Usage: "Verifying key", Required: true},
					&cli.UintFlag{Name: "inclusion-tree-height", Usage: "[Inclusion]: merkle tree height", Required: false},
					&cli.UintFlag{Name: "inclusion-compressed-accounts", Usage: "[Inclusion]: number of compressed accounts", Required: false},
					&cli.UintFlag{Name: "non-inclusion-tree-height", Usage: "[Non-inclusion]: merkle tree height", Required: false},
					&cli.UintFlag{Name: "non-inclusion-compressed-accounts", Usage: "[Non-inclusion]: number of compressed accounts", Required: false},
					&cli.UintFlag{Name: "append-tree-height", Usage: "[Batch append]: merkle tree height", Required: false},
					&cli.UintFlag{Name: "append-batch-size", Usage: "[Batch append]: batch size", Required: false},
					&cli.UintFlag{Name: "update-tree-height", Usage: "[Batch update]: merkle tree height", Required: false},
					&cli.UintFlag{Name: "update-batch-size", Usage: "[Batch update]: batch size", Required: false},
				},
				Action: func(context *cli.Context) error {
					circuit := context.String("circuit")
					if circuit != "inclusion" && circuit != "non-inclusion" && circuit != "combined" {
						return fmt.Errorf("invalid circuit type %s", circuit)
					}

					path := context.String("output")
					pk := context.String("pk")
					vk := context.String("vk")

					inclusionTreeHeight := uint32(context.Uint("inclusion-tree-height"))
					inclusionNumberOfCompressedAccounts := uint32(context.Uint("inclusion-compressed-accounts"))
					nonInclusionTreeHeight := uint32(context.Uint("non-inclusion-tree-height"))
					nonInclusionNumberOfCompressedAccounts := uint32(context.Uint("non-inclusion-compressed-accounts"))
					batchAppendTreeHeight := uint32(context.Uint("append-tree-height"))
					batchAppendBatchSize := uint32(context.Uint("append-batch-size"))
					batchUpdateTreeHeight := uint32(context.Uint("update-tree-height"))
					batchUpdateBatchSize := uint32(context.Uint("update-batch-size"))

					var err error

					logging.Logger().Info().Msg("Importing setup")

					if circuit == "append" {
						if batchAppendTreeHeight == 0 || batchAppendBatchSize == 0 {
							return fmt.Errorf("append tree height and batch size must be provided")
						}
						var system *prover.ProvingSystemV2
						system, err = prover.ImportBatchAppendSetup(batchAppendTreeHeight, batchAppendBatchSize, pk, vk)
						if err != nil {
							return err
						}
						err = writeProvingSystem(system, path, "")
					} else if circuit == "update" {
						if batchUpdateTreeHeight == 0 || batchUpdateBatchSize == 0 {
							return fmt.Errorf("append tree height and batch size must be provided")
						}
						var system *prover.ProvingSystemV2
						system, err = prover.ImportBatchUpdateSetup(batchUpdateTreeHeight, batchUpdateBatchSize, pk, vk)
						if err != nil {
							return err
						}
						err = writeProvingSystem(system, path, "")
					} else {
						if circuit == "inclusion" || circuit == "combined" {
							if inclusionTreeHeight == 0 || inclusionNumberOfCompressedAccounts == 0 {
								return fmt.Errorf("inclusion tree height and number of compressed accounts must be provided")
							}
						}
						if circuit == "non-inclusion" || circuit == "combined" {
							if nonInclusionTreeHeight == 0 || nonInclusionNumberOfCompressedAccounts == 0 {
								return fmt.Errorf("non-inclusion tree height and number of compressed accounts must be provided")
							}
						}

						var system *prover.ProvingSystemV1
						switch circuit {
						case "inclusion":
							system, err = prover.ImportInclusionSetup(inclusionTreeHeight, inclusionNumberOfCompressedAccounts, pk, vk)
						case "non-inclusion":
							system, err = prover.ImportNonInclusionSetup(nonInclusionTreeHeight, nonInclusionNumberOfCompressedAccounts, pk, vk)
						case "combined":
							system, err = prover.ImportCombinedSetup(inclusionTreeHeight, inclusionNumberOfCompressedAccounts, nonInclusionTreeHeight, nonInclusionNumberOfCompressedAccounts, pk, vk)
						}
						if err != nil {
							return err
						}
						err = writeProvingSystem(system, path, "")
					}

					if err != nil {
						return err
					}

					logging.Logger().Info().Msg("Setup imported successfully")
					return nil
				},
			},
			{
				Name: "export-vk",
				Flags: []cli.Flag{
					&cli.StringFlag{Name: "keys-file", Aliases: []string{"k"}, Usage: "proving system file", Required: true},
					&cli.StringFlag{Name: "output", Usage: "output file", Required: true},
				},
				Action: func(context *cli.Context) error {
					keysFile := context.String("keys-file")
					outputFile := context.String("output")

					system, err := prover.ReadSystemFromFile(keysFile)
					if err != nil {
						return fmt.Errorf("failed to read proving system: %v", err)
					}

					var vk interface{}
					switch s := system.(type) {
					case *prover.ProvingSystemV1:
						vk = s.VerifyingKey
					case *prover.ProvingSystemV2:
						vk = s.VerifyingKey
					default:
						return fmt.Errorf("unknown proving system type")
					}

					var buf bytes.Buffer
					_, err = vk.(io.WriterTo).WriteTo(&buf)
					if err != nil {
						return fmt.Errorf("failed to serialize verification key: %v", err)
					}

					err = os.MkdirAll(filepath.Dir(outputFile), 0755)
					if err != nil {
						return fmt.Errorf("failed to create output directory: %v", err)
					}

					var dataToWrite = buf.Bytes()

					err = os.WriteFile(outputFile, dataToWrite, 0644)
					if err != nil {
						return fmt.Errorf("failed to write verification key to file: %v", err)
					}

					logging.Logger().Info().
						Str("file", outputFile).
						Int("bytes", len(dataToWrite)).
						Msg("Verification key exported successfully")

					return nil
				},
			},
			{
				Name: "gen-test-params",
				Flags: []cli.Flag{
					&cli.IntFlag{Name: "tree-height", Usage: "height of the mock tree", DefaultText: "26", Value: 26},
					&cli.IntFlag{Name: "compressed-accounts", Usage: "Number of compressed accounts", DefaultText: "1", Value: 1},
				},
				Action: func(context *cli.Context) error {
					treeHeight := context.Int("tree-height")
					compressedAccounts := context.Int("compressed-accounts")
					logging.Logger().Info().Msg("Generating test params for the inclusion circuit")

					var r []byte
					var err error

					params := prover.BuildTestTree(treeHeight, compressedAccounts, false)

					r, err = json.Marshal(&params)

					if err != nil {
						return err
					}

					fmt.Println(string(r))
					return nil
				},
			},
			{
				Name: "start",
				Flags: []cli.Flag{
					&cli.BoolFlag{Name: "json-logging", Usage: "enable JSON logging", Required: false},
					&cli.StringFlag{Name: "prover-address", Usage: "address for the prover server", Value: "0.0.0.0:3001", Required: false},
					&cli.StringFlag{Name: "metrics-address", Usage: "address for the metrics server", Value: "0.0.0.0:9998", Required: false},
					&cli.BoolFlag{Name: "inclusion", Usage: "Run inclusion circuit", Required: false, Value: false},
					&cli.BoolFlag{Name: "non-inclusion", Usage: "Run non-inclusion circuit", Required: false},
					&cli.BoolFlag{Name: "append", Usage: "Run batch append circuit", Required: false},
					&cli.BoolFlag{Name: "update", Usage: "Run batch update circuit", Required: false},
					&cli.StringFlag{Name: "keys-dir", Usage: "Directory where key files are stored", Value: "./proving-keys/", Required: false},
					&cli.StringFlag{
						Name:  "run-mode",
						Usage: "Specify the running mode (test or full)",
						Value: "full",
					},
				},
				Action: func(context *cli.Context) error {
					if context.Bool("json-logging") {
						logging.SetJSONOutput()
					}

					runMode := context.String("run-mode")
					isTestMode := runMode == "test"

					if isTestMode {
						logging.Logger().Info().Msg("Running in test mode")
					} else {
						logging.Logger().Info().Msg("Running in full mode")
					}

					psv1, psv2, err := LoadKeys(context, isTestMode)
					if err != nil {
						return err
					}

					if len(psv1) == 0 && len(psv2) == 0 {
						return fmt.Errorf("no proving systems loaded")
					}

					merkleConfig := server.Config{
						ProverAddress:  context.String("prover-address"),
						MetricsAddress: context.String("metrics-address"),
					}
					instance := server.Run(&merkleConfig, psv1, psv2)
					sigint := make(chan os.Signal, 1)
					signal.Notify(sigint, os.Interrupt)
					<-sigint
					logging.Logger().Info().Msg("Received sigint, shutting down")
					instance.RequestStop()
					logging.Logger().Info().Msg("Waiting for server to close")
					instance.AwaitStop()
					return nil
				},
			},
			{
				Name: "prove",
				Flags: []cli.Flag{
					&cli.BoolFlag{Name: "inclusion", Usage: "Run inclusion circuit", Required: true},
					&cli.BoolFlag{Name: "non-inclusion", Usage: "Run non-inclusion circuit", Required: false},
					&cli.BoolFlag{Name: "append", Usage: "Run batch append circuit", Required: false},
					&cli.BoolFlag{Name: "update", Usage: "Run batch update circuit", Required: false},
					&cli.StringFlag{Name: "keys-dir", Usage: "Directory where circuit key files are stored", Value: "./proving-keys/", Required: false},
					&cli.StringSliceFlag{Name: "keys-file", Aliases: []string{"k"}, Value: cli.NewStringSlice(), Usage: "Proving system file"},
					&cli.StringFlag{
						Name:  "run-mode",
						Usage: "Specify the running mode (test or full)",
						Value: "full",
					},
				},
				Action: func(context *cli.Context) error {
					runMode := context.String("run-mode")
					isTestMode := runMode == "test"

					if isTestMode {
						logging.Logger().Info().Msg("Running in test mode")
					} else {
						logging.Logger().Info().Msg("Running in full mode")
					}

					psv1, psv2, err := LoadKeys(context, isTestMode)
					if err != nil {
						return err
					}

					if len(psv1) == 0 && len(psv2) == 0 {
						return fmt.Errorf("no proving systems loaded")
					}

					logging.Logger().Info().Msg("Reading params from stdin")
					inputsBytes, err := io.ReadAll(os.Stdin)
					if err != nil {
						return err
					}
					var proof *prover.Proof

					if context.Bool("inclusion") {
						var params prover.InclusionParameters
						err = json.Unmarshal(inputsBytes, &params)
						if err != nil {
							return err
						}

						treeHeight := params.TreeHeight()
						compressedAccounts := params.NumberOfCompressedAccounts()
						for _, provingSystem := range psv1 {
							if provingSystem.InclusionTreeHeight == treeHeight && provingSystem.InclusionNumberOfCompressedAccounts == compressedAccounts {
								proof, err = provingSystem.ProveInclusion(&params)
								if err != nil {
									return err
								}
								r, _ := json.Marshal(&proof)
								fmt.Println(string(r))
								break
							}
						}
					} else if context.Bool("non-inclusion") {
						var params prover.NonInclusionParameters
						err = json.Unmarshal(inputsBytes, &params)
						if err != nil {
							return err
						}

						treeHeight := params.TreeHeight()
						compressedAccounts := params.NumberOfCompressedAccounts()

						for _, provingSystem := range psv1 {
							if provingSystem.NonInclusionTreeHeight == treeHeight && provingSystem.NonInclusionNumberOfCompressedAccounts == compressedAccounts {
								proof, err = provingSystem.ProveNonInclusion(&params)
								if err != nil {
									return err
								}
								r, _ := json.Marshal(&proof)
								fmt.Println(string(r))
								break
							}
						}
					} else if context.Bool("inclusion") && context.Bool("non-inclusion") {
						var params prover.CombinedParameters
						err = json.Unmarshal(inputsBytes, &params)
						if err != nil {
							return err
						}

						for _, provingSystem := range psv1 {
							if provingSystem.InclusionTreeHeight == params.TreeHeight() && provingSystem.InclusionNumberOfCompressedAccounts == params.NumberOfCompressedAccounts() && provingSystem.NonInclusionTreeHeight == params.NonInclusionTreeHeight() && provingSystem.InclusionNumberOfCompressedAccounts == params.NonInclusionNumberOfCompressedAccounts() {
								proof, err = provingSystem.ProveCombined(&params)
								if err != nil {
									return err
								}
								r, _ := json.Marshal(&proof)
								fmt.Println(string(r))
								break
							}
						}
					} else if context.Bool("append") {
						var params prover.BatchAppendParameters
						err = json.Unmarshal(inputsBytes, &params)
						if err != nil {
							return err
						}

						for _, provingSystem := range psv2 {
							if provingSystem.TreeHeight == params.TreeHeight && provingSystem.BatchSize == params.BatchSize() {
								proof, err = provingSystem.ProveBatchAppend(&params)
								if err != nil {
									return err
								}
								r, _ := json.Marshal(&proof)
								fmt.Println(string(r))
								break
							}
						}
					} else if context.Bool("update") {
						var params prover.BatchUpdateParameters
						err = json.Unmarshal(inputsBytes, &params)
						if err != nil {
							return err
						}

						for _, provingSystem := range psv2 {
							if provingSystem.TreeHeight == params.Height && provingSystem.BatchSize == params.BatchSize {
								proof, err = provingSystem.ProveBatchUpdate(&params)
								if err != nil {
									return err
								}
								r, _ := json.Marshal(&proof)
								fmt.Println(string(r))
								break
							}
						}
					}

					return nil
				},
			},
			{
				Name: "verify",
				Flags: []cli.Flag{
					&cli.StringFlag{Name: "keys-file", Aliases: []string{"k"}, Usage: "proving system file", Required: true},
					&cli.StringFlag{Name: "circuit", Usage: "Type of circuit (\"inclusion\" / \"non-inclusion\" / \"combined\" / \"append\")", Required: true},
					&cli.StringFlag{Name: "roots", Usage: "Comma-separated list of root hashes (hex format)", Required: false},
					&cli.StringFlag{Name: "leaves", Usage: "Comma-separated list of leaf hashes for inclusion (hex format)", Required: false},
					&cli.StringFlag{Name: "values", Usage: "Comma-separated list of values for non-inclusion (hex format)", Required: false},
					&cli.StringFlag{Name: "old-sub-tree-hash-chain", Usage: "Old sub-tree hash chain (hex format)", Required: false},
					&cli.StringFlag{Name: "new-sub-tree-hash-chain", Usage: "New sub-tree hash chain (hex format)", Required: false},
					&cli.StringFlag{Name: "new-root", Usage: "New root (hex format)", Required: false},
					&cli.StringFlag{Name: "hashchain-hash", Usage: "Hashchain hash (hex format)", Required: false},
				},
				Action: func(context *cli.Context) error {
					keys := context.String("keys-file")
					circuit := context.String("circuit")

					system, err := prover.ReadSystemFromFile(keys)
					if err != nil {
						return fmt.Errorf("failed to read proving system: %v", err)
					}

					logging.Logger().Info().Msg("Reading proof from stdin")
					proofBytes, err := io.ReadAll(os.Stdin)
					if err != nil {
						return fmt.Errorf("failed to read proof from stdin: %v", err)
					}

					var proof prover.Proof
					err = json.Unmarshal(proofBytes, &proof)
					if err != nil {
						return fmt.Errorf("failed to unmarshal proof: %v", err)
					}

					var verifyErr error
					switch s := system.(type) {
					case *prover.ProvingSystemV1:
						rootsStr := context.String("roots")
						roots, err := parseHexStringList(rootsStr)
						if err != nil {
							return fmt.Errorf("failed to parse roots: %v", err)
						}

						switch circuit {
						case "inclusion":
							leavesStr := context.String("leaves")
							leaves, err := parseHexStringList(leavesStr)
							if err != nil {
								return fmt.Errorf("failed to parse leaves: %v", err)
							}

							verifyErr = s.VerifyInclusion(roots, leaves, &proof)
						case "non-inclusion":
							values, err := parseHexStringList(context.String("values"))
							if err != nil {
								return fmt.Errorf("failed to parse values: %v", err)
							}
							verifyErr = s.VerifyNonInclusion(roots, values, &proof)
						case "combined":
							leaves, err := parseHexStringList(context.String("leaves"))
							if err != nil {
								return fmt.Errorf("failed to parse leaves: %v", err)
							}
							values, err := parseHexStringList(context.String("values"))
							if err != nil {
								return fmt.Errorf("failed to parse values: %v", err)
							}
							verifyErr = s.VerifyCombined(roots, leaves, values, &proof)
						default:
							return fmt.Errorf("invalid circuit type for ProvingSystemV1: %s", circuit)
						}
					case *prover.ProvingSystemV2:
						if circuit != "append" {
							return fmt.Errorf("invalid circuit type for ProvingSystemV2: %s", circuit)
						}
						oldSubTreeHashChain, err := parseBigInt(context.String("old-sub-tree-hash-chain"))
						if err != nil {
							return fmt.Errorf("failed to parse old sub-tree hash chain: %v", err)
						}
						newSubTreeHashChain, err := parseBigInt(context.String("new-sub-tree-hash-chain"))
						if err != nil {
							return fmt.Errorf("failed to parse new sub-tree hash chain: %v", err)
						}
						newRoot, err := parseBigInt(context.String("new-root"))
						if err != nil {
							return fmt.Errorf("failed to parse new root: %v", err)
						}
						hashchainHash, err := parseBigInt(context.String("hashchain-hash"))
						if err != nil {
							return fmt.Errorf("failed to parse hashchain hash: %v", err)
						}
						verifyErr = s.VerifyBatchAppend(oldSubTreeHashChain, newSubTreeHashChain, newRoot, hashchainHash, &proof)
					default:
						return fmt.Errorf("unknown proving system type")
					}

					if verifyErr != nil {
						return fmt.Errorf("verification failed: %v", verifyErr)
					}

					logging.Logger().Info().Msg("Verification completed successfully")
					return nil

				},
			},
			{
				Name: "extract-circuit",
				Flags: []cli.Flag{
					&cli.StringFlag{Name: "output", Usage: "Output file", Required: true},
					&cli.UintFlag{Name: "tree-height", Usage: "Merkle tree height", Required: true},
					&cli.UintFlag{Name: "compressed-accounts", Usage: "Number of compressed accounts", Required: true},
				},
				Action: func(context *cli.Context) error {
					path := context.String("output")
					treeHeight := uint32(context.Uint("tree-height"))
					compressedAccounts := uint32(context.Uint("compressed-accounts"))
					logging.Logger().Info().Msg("Extracting gnark circuit to Lean")
					circuitString, err := prover.ExtractLean(treeHeight, compressedAccounts)
					if err != nil {
						return err
					}
					file, err := os.Create(path)
					defer func(file *os.File) {
						err := file.Close()
						if err != nil {
							logging.Logger().Error().Err(err).Msg("error closing file")
						}
					}(file)
					if err != nil {
						return err
					}
					written, err := file.WriteString(circuitString)
					if err != nil {
						return err
					}
					logging.Logger().Info().Int("bytesWritten", written).Msg("Lean circuit written to file")

					return nil
				},
			},
		},
	}

	if err := app.Run(os.Args); err != nil {
		logging.Logger().Fatal().Err(err).Msg("App failed.")
	}
}

func LoadKeys(context *cli.Context, isTestMode bool) ([]*prover.ProvingSystemV1, []*prover.ProvingSystemV2, error) {
	keys, _ := getKeysByArgs(context, isTestMode)
	var pssv1 []*prover.ProvingSystemV1
	var pssv2 []*prover.ProvingSystemV2

	for _, key := range keys {
		logging.Logger().Info().Msg("Reading proving system from file " + key + "...")
		system, err := prover.ReadSystemFromFile(key)
		if err != nil {
			return nil, nil, err
		}
		switch s := system.(type) {
		case *prover.ProvingSystemV1:
			pssv1 = append(pssv1, s)
			logging.Logger().Info().
				Uint32("inclusionTreeHeight", s.InclusionTreeHeight).
				Uint32("inclusionCompressedAccounts", s.InclusionNumberOfCompressedAccounts).
				Uint32("nonInclusionTreeHeight", s.NonInclusionTreeHeight).
				Uint32("nonInclusionCompressedAccounts", s.NonInclusionNumberOfCompressedAccounts).
				Msg("Read ProvingSystem")
		case *prover.ProvingSystemV2:
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

func getKeysByArgs(context *cli.Context, isTestMode bool) ([]string, error) {
	var keysDir = context.String("keys-dir")
	var inclusion = context.Bool("inclusion")
	var nonInclusion = context.Bool("non-inclusion")
	var batchAppend = context.Bool("append")
	var batchUpdate = context.Bool("update")
	var circuitTypes []prover.CircuitType = make([]prover.CircuitType, 0)

	if batchAppend {
		circuitTypes = append(circuitTypes, prover.BatchAppend)
	}

	if batchUpdate {
		circuitTypes = append(circuitTypes, prover.BatchUpdate)
	}

	if inclusion {
		circuitTypes = append(circuitTypes, prover.Inclusion)
	}

	if nonInclusion {
		circuitTypes = append(circuitTypes, prover.NonInclusion)
	}

	if inclusion && nonInclusion {
		circuitTypes = append(circuitTypes, prover.Combined)
	}

	return prover.GetKeys(keysDir, circuitTypes, isTestMode), nil
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

func writeProvingSystem(system interface{}, path string, pathVkey string) error {
	file, err := os.Create(path)
	if err != nil {
		return err
	}
	defer file.Close()

	var written int64
	switch s := system.(type) {
	case *prover.ProvingSystemV1:
		written, err = s.WriteTo(file)
	case *prover.ProvingSystemV2:
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
	case *prover.ProvingSystemV1:
		vk = s.VerifyingKey
	case *prover.ProvingSystemV2:
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

func parseHexStringList(input string) ([]big.Int, error) {
	hexStrings := strings.Split(input, ",")
	result := make([]big.Int, len(hexStrings))

	for i, hexString := range hexStrings {
		hexString = strings.TrimSpace(hexString)
		hexString = strings.TrimPrefix(hexString, "0x")

		bytes, err := hex.DecodeString(hexString)
		if err != nil {
			return nil, fmt.Errorf("invalid hex string: %s", hexString)
		}

		result[i].SetBytes(bytes)
	}

	return result, nil
}

func parseBigInt(input string) (*big.Int, error) {
	input = strings.TrimSpace(input)
	input = strings.TrimPrefix(input, "0x")

	bytes, err := hex.DecodeString(input)
	if err != nil {
		return nil, fmt.Errorf("invalid hex string: %s", input)
	}

	bigInt := new(big.Int).SetBytes(bytes)
	return bigInt, nil
}
