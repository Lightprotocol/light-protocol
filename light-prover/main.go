package main

import (
	"bytes"
	_ "embed"
	"encoding/json"
	"fmt"
	"io"
	"light/light-prover/logging"
	"light/light-prover/prover"
	"light/light-prover/server"
	"os"
	"os/signal"
	"path/filepath"

	"github.com/consensys/gnark/constraint"
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
					&cli.StringFlag{Name: "circuit", Usage: "Type of circuit (\"inclusion\" / \"non-inclusion\" / \"combined\" / \"append\" \"update\" )", Required: true},
					&cli.StringFlag{Name: "output", Usage: "Output file", Required: true},
					&cli.StringFlag{Name: "output-vkey", Usage: "Output file", Required: true},
					&cli.UintFlag{Name: "inclusion-tree-height", Usage: "[Inclusion]: Merkle tree height", Required: false},
					&cli.UintFlag{Name: "inclusion-compressed-accounts", Usage: "[Inclusion]: Number of compressed accounts", Required: false},
					&cli.UintFlag{Name: "non-inclusion-tree-height", Usage: "[Non-inclusion]: merkle tree height", Required: false},
					&cli.UintFlag{Name: "non-inclusion-compressed-accounts", Usage: "[Non-inclusion]: number of compressed accounts", Required: false},
					&cli.UintFlag{Name: "append-tree-height", Usage: "[Batch append]: tree height", Required: false},
					&cli.UintFlag{Name: "append-batch-size", Usage: "[Batch append]: batch size", Required: false},
					&cli.UintFlag{Name: "update-tree-height", Usage: "[Batch update]: tree height", Required: false},
					&cli.UintFlag{Name: "update-batch-size", Usage: "[Batch update]: batch size", Required: false},
				},
				Action: func(context *cli.Context) error {
					circuit := prover.CircuitType(context.String("circuit"))
					if circuit != prover.InclusionCircuitType && circuit != prover.NonInclusionCircuitType && circuit != prover.CombinedCircuitType && circuit != prover.BatchAppendCircuitType && circuit != prover.BatchUpdateCircuitType && circuit != prover.BatchAppend2CircuitType {
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

					if (inclusionTreeHeight == 0 || inclusionNumberOfCompressedAccounts == 0) && circuit == prover.InclusionCircuitType {
						return fmt.Errorf("inclusion tree height and number of compressed accounts must be provided")
					}

					if (nonInclusionTreeHeight == 0 || nonInclusionNumberOfCompressedAccounts == 0) && circuit == prover.NonInclusionCircuitType {
						return fmt.Errorf("non-inclusion tree height and number of compressed accounts must be provided")
					}

					if circuit == prover.CombinedCircuitType {
						if inclusionTreeHeight == 0 || inclusionNumberOfCompressedAccounts == 0 {
							return fmt.Errorf("inclusion tree height and number of compressed accounts must be provided")
						}
						if nonInclusionTreeHeight == 0 || nonInclusionNumberOfCompressedAccounts == 0 {
							return fmt.Errorf("non-inclusion tree height and number of compressed accounts must be provided")
						}
					}

					if (batchAppendTreeHeight == 0 || batchAppendBatchSize == 0) && circuit == prover.BatchAppendCircuitType {
						return fmt.Errorf("[Batch append]: tree height and batch size must be provided")
					}

					if (batchUpdateTreeHeight == 0 || batchUpdateBatchSize == 0) && circuit == prover.BatchUpdateCircuitType {
						return fmt.Errorf("[Batch update]: tree height and batch size must be provided")
					}
					logging.Logger().Info().Msg("Running setup")
					var err error
					if circuit == prover.BatchAppendCircuitType {
						var system *prover.ProvingSystemV2
						system, err = prover.SetupCircuitV2(prover.BatchAppendCircuitType, batchAppendTreeHeight, batchAppendBatchSize)
						if err != nil {
							return err
						}
						err = prover.WriteProvingSystem(system, path, pathVkey)
					} else if circuit == prover.BatchAppend2CircuitType {
						var system *prover.ProvingSystemV2
						system, err = prover.SetupCircuitV2(prover.BatchAppend2CircuitType, batchAppendTreeHeight, batchAppendBatchSize)
						if err != nil {
							return err
						}
						err = prover.WriteProvingSystem(system, path, pathVkey)
					} else if circuit == prover.BatchUpdateCircuitType {
						var system *prover.ProvingSystemV2
						system, err = prover.SetupCircuitV2(prover.BatchUpdateCircuitType, batchUpdateTreeHeight, batchUpdateBatchSize)
						if err != nil {
							return err
						}
						err = prover.WriteProvingSystem(system, path, pathVkey)
					} else {
						var system *prover.ProvingSystemV1
						system, err = prover.SetupCircuitV1(circuit, inclusionTreeHeight, inclusionNumberOfCompressedAccounts, nonInclusionTreeHeight, nonInclusionNumberOfCompressedAccounts)
						if err != nil {
							return err
						}
						err = prover.WriteProvingSystem(system, path, pathVkey)
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
					if circuit != prover.InclusionCircuitType && circuit != prover.NonInclusionCircuitType && circuit != prover.CombinedCircuitType && circuit != prover.BatchAppendCircuitType {
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

					if (batchAppendTreeHeight == 0 || batchAppendBatchSize == 0) && circuit == prover.BatchAppendCircuitType {
						return fmt.Errorf("[Batch append]: tree height and batch size must be provided")
					}

					if (batchUpdateTreeHeight == 0 || batchUpdateBatchSize == 0) && circuit == prover.BatchUpdateCircuitType {
						return fmt.Errorf("[Batch update]: tree height and batch size must be provided")
					}

					logging.Logger().Info().Msg("Building R1CS")

					var cs constraint.ConstraintSystem
					var err error

					if circuit == prover.InclusionCircuitType {
						cs, err = prover.R1CSInclusion(inclusionTreeHeight, inclusionNumberOfCompressedAccounts)
					} else if circuit == prover.NonInclusionCircuitType {
						cs, err = prover.R1CSNonInclusion(nonInclusionTreeHeight, nonInclusionNumberOfCompressedAccounts)
					} else if circuit == prover.CombinedCircuitType {
						cs, err = prover.R1CSCombined(inclusionTreeHeight, inclusionNumberOfCompressedAccounts, nonInclusionTreeHeight, nonInclusionNumberOfCompressedAccounts)
					} else if circuit == prover.BatchAppendCircuitType {
						cs, err = prover.R1CSBatchAppend(batchAppendTreeHeight, batchAppendBatchSize)
					} else if circuit == prover.BatchUpdateCircuitType {
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
						err = prover.WriteProvingSystem(system, path, "")
					} else if circuit == "update" {
						if batchUpdateTreeHeight == 0 || batchUpdateBatchSize == 0 {
							return fmt.Errorf("append tree height and batch size must be provided")
						}
						var system *prover.ProvingSystemV2
						system, err = prover.ImportBatchUpdateSetup(batchUpdateTreeHeight, batchUpdateBatchSize, pk, vk)
						if err != nil {
							return err
						}
						err = prover.WriteProvingSystem(system, path, "")
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
						err = prover.WriteProvingSystem(system, path, "")
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
					&cli.StringFlag{Name: "keys-dir", Usage: "Directory where key files are stored", Value: "./proving-keys/", Required: false},
					&cli.StringSliceFlag{
						Name:  "circuit",
						Usage: "Specify the circuits to enable (inclusion, non-inclusion, combined, append, update, append-test, append2-test, update-test)",
					},
					&cli.StringFlag{
						Name:  "run-mode",
						Usage: "Specify the running mode (rpc, forester, forester-test, full, or full-test)",
					},
				},
				Action: func(context *cli.Context) error {
					if context.Bool("json-logging") {
						logging.SetJSONOutput()
					}

					circuits := context.StringSlice("circuit")
					runMode, err := parseRunMode(context.String("run-mode"))
					if err != nil {
						if len(circuits) == 0 {
							return err
						}
					}

					var keysDirPath = context.String("keys-dir")

					psv1, psv2, err := prover.LoadKeys(keysDirPath, runMode, circuits)
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
					&cli.StringSliceFlag{
						Name:  "circuit",
						Usage: "Specify the circuits to enable (inclusion, non-inclusion, combined, append, update, append-test, append2-test, update-test)",
						Value: cli.NewStringSlice("inclusion", "non-inclusion", "combined", "append", "update", "append-test", "append2-test", "update-test"),
					},
					&cli.StringFlag{
						Name:  "run-mode",
						Usage: "Specify the running mode (forester, forester-test, rpc, or full)",
					},
				},
				Action: func(context *cli.Context) error {
					circuits := context.StringSlice("circuit")
					runMode, err := parseRunMode(context.String("run-mode"))
					if err != nil {
						if len(circuits) == 0 {
							return err
						}
					}
					var keysDirPath = context.String("keys-dir")

					psv1, psv2, err := prover.LoadKeys(keysDirPath, runMode, circuits)
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
						roots, err := prover.ParseHexStringList(rootsStr)
						if err != nil {
							return fmt.Errorf("failed to parse roots: %v", err)
						}

						switch circuit {
						case "inclusion":
							leavesStr := context.String("leaves")
							leaves, err := prover.ParseHexStringList(leavesStr)
							if err != nil {
								return fmt.Errorf("failed to parse leaves: %v", err)
							}

							verifyErr = s.VerifyInclusion(roots, leaves, &proof)
						case "non-inclusion":
							values, err := prover.ParseHexStringList(context.String("values"))
							if err != nil {
								return fmt.Errorf("failed to parse values: %v", err)
							}
							verifyErr = s.VerifyNonInclusion(roots, values, &proof)
						case "combined":
							leaves, err := prover.ParseHexStringList(context.String("leaves"))
							if err != nil {
								return fmt.Errorf("failed to parse leaves: %v", err)
							}
							values, err := prover.ParseHexStringList(context.String("values"))
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
						oldSubTreeHashChain, err := prover.ParseBigInt(context.String("old-sub-tree-hash-chain"))
						if err != nil {
							return fmt.Errorf("failed to parse old sub-tree hash chain: %v", err)
						}
						newSubTreeHashChain, err := prover.ParseBigInt(context.String("new-sub-tree-hash-chain"))
						if err != nil {
							return fmt.Errorf("failed to parse new sub-tree hash chain: %v", err)
						}
						newRoot, err := prover.ParseBigInt(context.String("new-root"))
						if err != nil {
							return fmt.Errorf("failed to parse new root: %v", err)
						}
						hashchainHash, err := prover.ParseBigInt(context.String("hashchain-hash"))
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

func parseRunMode(runModeString string) (prover.RunMode, error) {
	runMode := prover.Rpc
	switch runModeString {
	case "rpc":
		logging.Logger().Info().Msg("Running in rpc mode")
		runMode = prover.Rpc
	case "forester":
		logging.Logger().Info().Msg("Running in forester mode")
		runMode = prover.Forester
	case "forester-test":
		logging.Logger().Info().Msg("Running in forester test mode")
		runMode = prover.ForesterTest
	case "full":
		logging.Logger().Info().Msg("Running in full mode")
		runMode = prover.Full
	case "full-test":
		logging.Logger().Info().Msg("Running in full mode")
		runMode = prover.FullTest
	default:
		return "", fmt.Errorf("invalid run mode %s", runModeString)
	}
	return runMode, nil
}
