package main

import (
	"bytes"
	_ "embed"
	"encoding/json"
	"fmt"
	"io"
	"light/light-prover/logging"
	merkletree "light/light-prover/merkle-tree"
	"light/light-prover/prover"
	"light/light-prover/server"
	"math/big"
	"os"
	"os/signal"
	"strings"

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
					&cli.StringFlag{Name: "circuit", Usage: "Type of circuit (\"inclusion\" / \"non-inclusion\" / \"combined\")", Required: true},
					&cli.StringFlag{Name: "output", Usage: "Output file", Required: true},
					&cli.StringFlag{Name: "output-vkey", Usage: "Output file", Required: true},
					&cli.UintFlag{Name: "inclusion-tree-height", Usage: "Merkle tree height", Required: false},
					&cli.UintFlag{Name: "inclusion-compressed-accounts", Usage: "Number of compressed accounts", Required: false},
					&cli.UintFlag{Name: "non-inclusion-tree-height", Usage: "Non-inclusion merkle tree height", Required: false},
					&cli.UintFlag{Name: "non-inclusion-compressed-accounts", Usage: "Non-inclusion number of compressed accounts", Required: false},
				},
				Action: func(context *cli.Context) error {
					circuit := prover.CircuitType(context.String("circuit"))
					if circuit != prover.Inclusion && circuit != prover.NonInclusion && circuit != prover.Combined {
						return fmt.Errorf("invalid circuit type %s", circuit)
					}

					path := context.String("output")
					pathVkey := context.String("output-vkey")
					inclusionTreeHeight := uint32(context.Uint("inclusion-tree-height"))
					inclusionNumberOfCompressedAccounts := uint32(context.Uint("inclusion-compressed-accounts"))
					nonInclusionTreeHeight := uint32(context.Uint("non-inclusion-tree-height"))
					nonInclusionNumberOfCompressedAccounts := uint32(context.Uint("non-inclusion-compressed-accounts"))

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

					logging.Logger().Info().Msg("Running setup")

					var system *prover.ProvingSystem
					var err error
					system, err = prover.SetupCircuit(circuit, inclusionTreeHeight, inclusionNumberOfCompressedAccounts, nonInclusionTreeHeight, nonInclusionNumberOfCompressedAccounts)
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
					written, err := system.WriteTo(file)
					if err != nil {
						return err
					}
					logging.Logger().Info().Int64("bytesWritten", written).Msg("proving system written to file")
					ps, err := prover.ReadSystemFromFile(path)
					if err != nil {
						return err
					}
					var buf bytes.Buffer
					_, err = ps.VerifyingKey.WriteRawTo(&buf)

					proofBytes := buf.Bytes()
					err = createFileAndWriteBytes(pathVkey, proofBytes)
					if err != nil {
						return err
					}
					return nil
				},
			},
			{
				Name: "r1cs",
				Flags: []cli.Flag{
					&cli.StringFlag{Name: "output", Usage: "Output file", Required: true},
					&cli.StringFlag{Name: "circuit", Usage: "Type of circuit (\"inclusion\" / \"non-inclusion\" / \"combined\")", Required: true},
					&cli.UintFlag{Name: "inclusion-tree-height", Usage: "Merkle tree height", Required: false},
					&cli.UintFlag{Name: "inclusion-compressed-accounts", Usage: "Number of compressed accounts", Required: false},
					&cli.UintFlag{Name: "non-inclusion-tree-height", Usage: "Non-inclusion merkle tree height", Required: false},
					&cli.UintFlag{Name: "non-inclusion-compressed-accounts", Usage: "Non-inclusion number of compressed accounts", Required: false},
				},
				Action: func(context *cli.Context) error {
					circuit := context.String("circuit")
					if circuit != "inclusion" && circuit != "non-inclusion" && circuit != "combined" {
						return fmt.Errorf("invalid circuit type %s", circuit)
					}

					path := context.String("output")
					inclusionTreeHeight := uint32(context.Uint("inclusion-tree-height"))
					inclusionNumberOfCompressedAccounts := uint32(context.Uint("inclusion-compressed-accounts"))
					nonInclusionTreeHeight := uint32(context.Uint("non-inclusion-tree-height"))
					nonInclusionNumberOfCompressedAccounts := uint32(context.Uint("non-inclusion-compressed-accounts"))

					if (inclusionTreeHeight == 0 || inclusionNumberOfCompressedAccounts == 0) && circuit == "inclusion" {
						return fmt.Errorf("inclusion tree height and number of compressed accounts must be provided")
					}

					if (nonInclusionTreeHeight == 0 || nonInclusionNumberOfCompressedAccounts == 0) && circuit == "non-inclusion" {
						return fmt.Errorf("non-inclusion tree height and number of compressed accounts must be provided")
					}

					if circuit == "combined" {
						if inclusionTreeHeight == 0 || inclusionNumberOfCompressedAccounts == 0 {
							return fmt.Errorf("inclusion tree height and number of compressed accounts must be provided")
						}
						if nonInclusionTreeHeight == 0 || nonInclusionNumberOfCompressedAccounts == 0 {
							return fmt.Errorf("non-inclusion tree height and number of compressed accounts must be provided")
						}
					}

					logging.Logger().Info().Msg("Building R1CS")

					var cs constraint.ConstraintSystem
					var err error

					if circuit == "inclusion" {
						cs, err = prover.R1CSInclusion(inclusionTreeHeight, inclusionNumberOfCompressedAccounts)
					} else if circuit == "non-inclusion" {
						cs, err = prover.R1CSNonInclusion(nonInclusionTreeHeight, nonInclusionNumberOfCompressedAccounts)
					} else if circuit == "combined" {
						cs, err = prover.R1CSCombined(inclusionTreeHeight, inclusionNumberOfCompressedAccounts, nonInclusionTreeHeight, nonInclusionNumberOfCompressedAccounts)
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
					&cli.UintFlag{Name: "inclusion-tree-height", Usage: "Merkle tree height", Required: false},
					&cli.UintFlag{Name: "inclusion-compressed-accounts", Usage: "Number of compressed accounts", Required: false},
					&cli.UintFlag{Name: "non-inclusion-tree-height", Usage: "Non-inclusion merkle tree height", Required: false},
					&cli.UintFlag{Name: "non-inclusion-compressed-accounts", Usage: "Non-inclusion number of compressed accounts", Required: false},
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

					if (inclusionTreeHeight == 0 || inclusionNumberOfCompressedAccounts == 0) && circuit == "inclusion" {
						return fmt.Errorf("inclusion tree height and number of compressed accounts must be provided")
					}

					if (nonInclusionTreeHeight == 0 || nonInclusionNumberOfCompressedAccounts == 0) && circuit == "non-inclusion" {
						return fmt.Errorf("non-inclusion tree height and number of compressed accounts must be provided")
					}

					if circuit == "combined" {
						if inclusionTreeHeight == 0 || inclusionNumberOfCompressedAccounts == 0 {
							return fmt.Errorf("inclusion tree height and number of compressed accounts must be provided")
						}
						if nonInclusionTreeHeight == 0 || nonInclusionNumberOfCompressedAccounts == 0 {
							return fmt.Errorf("non-inclusion tree height and number of compressed accounts must be provided")
						}
					}

					var system *prover.ProvingSystem
					var err error

					logging.Logger().Info().Msg("Importing setup")

					if circuit == "inclusion" {
						system, err = prover.ImportInclusionSetup(inclusionTreeHeight, inclusionNumberOfCompressedAccounts, pk, vk)
					} else if circuit == "non-inclusion" {
						system, err = prover.ImportNonInclusionSetup(nonInclusionTreeHeight, nonInclusionNumberOfCompressedAccounts, pk, vk)
					} else if circuit == "combined" {
						system, err = prover.ImportCombinedSetup(inclusionTreeHeight, inclusionNumberOfCompressedAccounts, nonInclusionTreeHeight, nonInclusionNumberOfCompressedAccounts, pk, vk)
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
					written, err := system.WriteTo(file)
					if err != nil {
						return err
					}
					logging.Logger().Info().Int64("bytesWritten", written).Msg("proving system written to file")
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
					keys := context.String("keys-file")
					ps, err := prover.ReadSystemFromFile(keys)
					if err != nil {
						return err
					}
					outPath := context.String("output")

					file, err := os.Create(outPath)
					defer func(file *os.File) {
						err := file.Close()
						if err != nil {
							logging.Logger().Error().Err(err).Msg("error closing file")
						}
					}(file)
					if err != nil {
						return err
					}
					output := file
					_, err = ps.VerifyingKey.WriteTo(output)
					return err
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

					params := merkletree.BuildTestTree(treeHeight, compressedAccounts, false)

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
					&cli.StringFlag{Name: "keys-dir", Usage: "Directory where key files are stored", Value: "./proving-keys/", Required: false},
				},
				Action: func(context *cli.Context) error {
					if context.Bool("json-logging") {
						logging.SetJSONOutput()
					}

					ps, err := LoadKeys(context)
					if err != nil {
						return err
					}
					if len(ps) == 0 {
						return fmt.Errorf("no proving systems loaded")
					}

					merkleConfig := server.Config{
						ProverAddress:  context.String("prover-address"),
						MetricsAddress: context.String("metrics-address"),
					}
					instance := server.Run(&merkleConfig, ps)
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
					&cli.StringFlag{Name: "keys-dir", Usage: "Directory where circuit key files are stored", Value: "./proving-keys/", Required: false},
					&cli.StringSliceFlag{Name: "keys-file", Aliases: []string{"k"}, Value: cli.NewStringSlice(), Usage: "Proving system file"},
				},
				Action: func(context *cli.Context) error {
					ps, err := LoadKeys(context)
					if err != nil {
						return err
					}

					if len(ps) == 0 {
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
						for _, provingSystem := range ps {
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

						for _, provingSystem := range ps {
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

						for _, provingSystem := range ps {
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
					}

					return nil
				},
			},
			{
				Name: "verify",
				Flags: []cli.Flag{
					&cli.StringFlag{Name: "keys-file", Aliases: []string{"k"}, Usage: "proving system file", Required: true},
					&cli.StringFlag{Name: "roots", Usage: "array of roots", Required: true},
					&cli.StringFlag{Name: "leafs", Usage: "array of leafs", Required: true},
				},
				Action: func(context *cli.Context) error {
					keys := context.String("keys-file")

					rootInputString := context.String("roots")
					rootStrings := strings.Split(rootInputString, ",")
					roots := make([]big.Int, len(rootStrings))

					for i, rootString := range rootStrings {
						rootString = strings.ToLower(strings.TrimSpace(rootString))
						rootString = strings.TrimPrefix(rootString, "0x")
						val := new(big.Int)
						val.SetString(rootString, 16)
						roots[i] = *val
					}

					leafInputString := context.String("leafs")
					leafStrings := strings.Split(leafInputString, ",")
					leafs := make([]big.Int, len(leafStrings))

					for i, leafString := range leafStrings {
						leafString = strings.ToLower(strings.TrimSpace(leafString))
						leafString = strings.TrimPrefix(leafString, "0x")
						val := new(big.Int)
						val.SetString(leafString, 16)
						leafs[i] = *val
					}

					ps, err := prover.ReadSystemFromFile(keys)
					if err != nil {
						return err
					}
					logging.Logger().Info().
						Uint32("treeHeight", ps.InclusionTreeHeight).
						Uint32("compressedAccounts", ps.InclusionNumberOfCompressedAccounts).
						Uint32("nonInclusionTreeHeight", ps.NonInclusionTreeHeight).
						Uint32("nonInclusionCompressedAccounts", ps.NonInclusionNumberOfCompressedAccounts).
						Msg("Read proving system")
					logging.Logger().Info().Msg("Reading proof from stdin")
					proofBytes, err := io.ReadAll(os.Stdin)
					if err != nil {
						logging.Logger().Err(err).Msg("error reading proof from stdin")
						return err
					}
					logging.Logger().Info().Msg("Parsing proof from stdin")
					var proof prover.Proof
					err = json.Unmarshal(proofBytes, &proof)
					if err != nil {
						logging.Logger().Err(err).Msg("error unmarshalling proof from stdin")
						return err
					}
					logging.Logger().Info().Msg("Proof read successfully")
					err = ps.VerifyInclusion(roots, leafs, &proof)
					if err != nil {
						return err
					}
					logging.Logger().Info().Msg("verification complete")
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

func LoadKeys(context *cli.Context) ([]*prover.ProvingSystem, error) {
	keys, _ := getKeysByArgs(context)
	var pss = make([]*prover.ProvingSystem, len(keys))
	for i, key := range keys {
		logging.Logger().Info().Msg("Reading proving system from file " + key + "...")
		ps, err := prover.ReadSystemFromFile(key)
		if err != nil {
			return nil, err
		}
		pss[i] = ps
		logging.Logger().Info().
			Uint32("treeHeight", ps.InclusionTreeHeight).
			Uint32("compressedAccounts", ps.InclusionNumberOfCompressedAccounts).
			Uint32("nonInclusionTreeHeight", ps.NonInclusionTreeHeight).
			Uint32("nonInclusionCompressedAccounts", ps.NonInclusionNumberOfCompressedAccounts).
			Msg("Read proving system")
	}
	return pss, nil
}

func getKeysByArgs(context *cli.Context) ([]string, error) {
	var keysDir = context.String("keys-dir")
	var inclusion = context.Bool("inclusion")
	var nonInclusion = context.Bool("non-inclusion")
	var circuitTypes = make([]prover.CircuitType, 0)
	if inclusion {
		circuitTypes = append(circuitTypes, prover.Inclusion)
	}

	if nonInclusion {
		circuitTypes = append(circuitTypes, prover.NonInclusion)
	}

	if inclusion && nonInclusion {
		circuitTypes = append(circuitTypes, prover.Combined)
	}

	if !inclusion && !nonInclusion {
		return nil, fmt.Errorf("no circuit type provided")
	}

	return prover.GetKeys(keysDir, circuitTypes), nil
}

func createFileAndWriteBytes(filePath string, data []byte) error {
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
