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
	"time"

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
					&cli.StringFlag{Name: "circuit", Usage: "Type of circuit (\"inclusion\" / \"non-inclusion\" / \"combined\" / \"append\" / \"update\" / \"address-append\" )", Required: true},
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
					&cli.UintFlag{Name: "address-append-tree-height", Usage: "[Batch address append]: tree height", Required: false},
					&cli.UintFlag{Name: "address-append-batch-size", Usage: "[Batch address append]: batch size", Required: false},
				},
				Action: func(context *cli.Context) error {
					circuit := prover.CircuitType(context.String("circuit"))
					if circuit != prover.InclusionCircuitType && circuit != prover.NonInclusionCircuitType && circuit != prover.CombinedCircuitType && circuit != prover.BatchUpdateCircuitType && circuit != prover.BatchAppendCircuitType && circuit != prover.BatchAddressAppendCircuitType {
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
					batchAddressAppendTreeHeight := uint32(context.Uint("address-append-tree-height"))
					batchAddressAppendBatchSize := uint32(context.Uint("address-append-batch-size"))

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

					if (batchUpdateTreeHeight == 0 || batchUpdateBatchSize == 0) && circuit == prover.BatchUpdateCircuitType {
						return fmt.Errorf("[Batch update]: tree height and batch size must be provided")
					}

					if (batchAddressAppendTreeHeight == 0 || batchAddressAppendBatchSize == 0) && circuit == prover.BatchAddressAppendCircuitType {
						return fmt.Errorf("[Batch address append]: tree height and batch size must be provided")
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
					} else if circuit == prover.BatchUpdateCircuitType {
						var system *prover.ProvingSystemV2
						system, err = prover.SetupCircuitV2(prover.BatchUpdateCircuitType, batchUpdateTreeHeight, batchUpdateBatchSize)
						if err != nil {
							return err
						}
						err = prover.WriteProvingSystem(system, path, pathVkey)
					} else if circuit == prover.BatchAddressAppendCircuitType {
						fmt.Println("Generating Address Append Circuit")
						var system *prover.ProvingSystemV2
						system, err = prover.SetupCircuitV2(prover.BatchAddressAppendCircuitType, batchAddressAppendTreeHeight, batchAddressAppendBatchSize)
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
					&cli.UintFlag{Name: "address-append-tree-height", Usage: "[Batch address append]: tree height", Required: false},
					&cli.UintFlag{Name: "address-append-batch-size", Usage: "[Batch address append]: batch size", Required: false},
				},
				Action: func(context *cli.Context) error {
					circuit := prover.CircuitType(context.String("circuit"))
					if circuit != prover.InclusionCircuitType &&
						circuit != prover.NonInclusionCircuitType &&
						circuit != prover.CombinedCircuitType &&
						circuit != prover.BatchUpdateCircuitType &&
						circuit != prover.BatchAppendCircuitType &&
						circuit != prover.BatchAddressAppendCircuitType {
						return fmt.Errorf("invalid circuit type %s", circuit)
					}

					path := context.String("output")
					inclusionTreeHeight := uint32(context.Uint("inclusion-tree-height"))
					inclusionNumberOfCompressedAccounts := uint32(context.Uint("inclusion-compressed-accounts"))
					nonInclusionTreeHeight := uint32(context.Uint("non-inclusion-tree-height"))
					nonInclusionNumberOfCompressedAccounts := uint32(context.Uint("non-inclusion-compressed-accounts"))
					batchUpdateTreeHeight := uint32(context.Uint("update-tree-height"))
					batchUpdateBatchSize := uint32(context.Uint("update-batch-size"))
					batchAddressAppendTreeHeight := uint32(context.Uint("address-append-tree-height"))
					batchAddressAppendBatchSize := uint32(context.Uint("address-append-batch-size"))

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

					if (batchUpdateTreeHeight == 0 || batchUpdateBatchSize == 0) && circuit == prover.BatchUpdateCircuitType {
						return fmt.Errorf("[Batch update]: tree height and batch size must be provided")
					}

					if (batchAddressAppendTreeHeight == 0 || batchAddressAppendBatchSize == 0) && circuit == prover.BatchAddressAppendCircuitType {
						return fmt.Errorf("[Batch address append]: tree height and batch size must be provided")
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
					} else if circuit == prover.BatchUpdateCircuitType {
						cs, err = prover.R1CSBatchUpdate(batchUpdateTreeHeight, batchUpdateBatchSize)
					} else if circuit == prover.BatchAddressAppendCircuitType {
						cs, err = prover.R1CSBatchAddressAppend(batchAddressAppendTreeHeight, batchAddressAppendBatchSize)
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
					&cli.UintFlag{Name: "address-append-tree-height", Usage: "[Batch address append]: tree height", Required: false},
					&cli.UintFlag{Name: "address-append-batch-size", Usage: "[Batch address append]: batch size", Required: false},
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
					batchAddressAppendTreeHeight := uint32(context.Uint("address-append-tree-height"))
					batchAddressAppendBatchSize := uint32(context.Uint("address-append-batch-size"))

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
					} else if circuit == "address-append" {
						if batchAddressAppendTreeHeight == 0 || batchAddressAppendBatchSize == 0 {
							return fmt.Errorf("append tree height and batch size must be provided")
						}
						var system *prover.ProvingSystemV2
						system, err = prover.ImportBatchAddressAppendSetup(batchAddressAppendTreeHeight, batchAddressAppendBatchSize, pk, vk)
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
						Usage: "Specify the circuits to enable (inclusion, non-inclusion, combined, append, update, append-test, update-test, address-append, address-append-test)",
					},
					&cli.StringFlag{
						Name:  "run-mode",
						Usage: "Specify the running mode (rpc, forester, forester-test, full, or full-test)",
					},
					&cli.StringFlag{
						Name:  "redis-url",
						Usage: "Redis URL for queue processing (e.g., redis://localhost:6379)",
						Value: "",
					},
					&cli.BoolFlag{
						Name:  "queue-only",
						Usage: "Run only queue workers (no HTTP server)",
						Value: false,
					},
					&cli.BoolFlag{
						Name:  "server-only",
						Usage: "Run only HTTP server (no queue workers)",
						Value: false,
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
					debugProvingSystemKeys(keysDirPath, runMode, circuits)
					psv1, psv2, err := prover.LoadKeys(keysDirPath, runMode, circuits)
					if err != nil {
						return err
					}

					if len(psv1) == 0 && len(psv2) == 0 {
						return fmt.Errorf("no proving systems loaded")
					}

					redisURL := context.String("redis-url")
					if redisURL == "" {
						redisURL = os.Getenv("REDIS_URL")
					}

					queueOnly := context.Bool("queue-only")
					serverOnly := context.Bool("server-only")

					enableQueue := redisURL != "" && !serverOnly
					enableServer := !queueOnly

					if os.Getenv("QUEUE_MODE") == "true" {
						enableQueue = true
						if os.Getenv("SERVER_MODE") != "true" {
							enableServer = false
						}
					}

					logging.Logger().Info().
						Bool("enable_queue", enableQueue).
						Bool("enable_server", enableServer).
						Str("redis_url", redisURL).
						Msg("Starting ZK Prover service")

					var workers []server.QueueWorker
					var redisQueue *server.RedisQueue
					var instance server.RunningJob

					if enableQueue {
						if redisURL == "" {
							return fmt.Errorf("Redis URL is required for queue mode. Use --redis-url or set REDIS_URL environment variable")
						}

						redisQueue, err = server.NewRedisQueue(redisURL)
						if err != nil {
							return fmt.Errorf("failed to connect to Redis: %w", err)
						}

						startCleanupRoutines(redisQueue)

						if stats, err := redisQueue.GetQueueStats(); err == nil {
							logging.Logger().Info().Interface("initial_queue_stats", stats).Msg("Redis connection successful")
						}

						logging.Logger().Info().Msg("Starting queue workers")

						startAllWorkers := runMode == prover.Forester || runMode == prover.ForesterTest || runMode == prover.Full || runMode == prover.FullTest

						var workersStarted []string

						logging.Logger().Info().Bool("startAllWorkers", startAllWorkers)

						for _, circuit := range circuits {
							logging.Logger().Info().Str("circuit", circuit)
						}
						// Start update worker for batch-update circuits or forester modes
						if startAllWorkers || containsCircuit(circuits, "update") || containsCircuit(circuits, "update-test") {
							updateWorker := server.NewUpdateQueueWorker(redisQueue, psv1, psv2)
							workers = append(workers, updateWorker)
							go updateWorker.Start()
							workersStarted = append(workersStarted, "update")
						}

						// Start append worker for batch-append circuits or forester modes
						if startAllWorkers || containsCircuit(circuits, "append") || containsCircuit(circuits, "append-test") {
							appendWorker := server.NewAppendQueueWorker(redisQueue, psv1, psv2)
							workers = append(workers, appendWorker)
							go appendWorker.Start()
							workersStarted = append(workersStarted, "append")
						}

						// Start address append worker for address-append circuits or forester modes
						if startAllWorkers || containsCircuit(circuits, "address-append") || containsCircuit(circuits, "address-append-test") {
							addressAppendWorker := server.NewAddressAppendQueueWorker(redisQueue, psv1, psv2)
							workers = append(workers, addressAppendWorker)
							go addressAppendWorker.Start()
							workersStarted = append(workersStarted, "address-append")
						}

						if len(workersStarted) == 0 {
							logging.Logger().Warn().Msg("No queue workers started - no matching circuits found")
						} else {
							logging.Logger().Info().
								Strs("workers_started", workersStarted).
								Bool("forester_mode", startAllWorkers).
								Msg("Queue workers started")
						}
					}

					if enableServer {
						config := server.Config{
							ProverAddress:  context.String("prover-address"),
							MetricsAddress: context.String("metrics-address"),
						}

						if redisQueue != nil {
							instance = server.RunWithQueue(&config, redisQueue, circuits, runMode, psv1, psv2)
							logging.Logger().Info().
								Str("prover_address", config.ProverAddress).
								Str("metrics_address", config.MetricsAddress).
								Msg("Started enhanced server with Redis queue support")
						} else {
							instance = server.Run(&config, circuits, runMode, psv1, psv2)
							logging.Logger().Info().
								Str("prover_address", config.ProverAddress).
								Str("metrics_address", config.MetricsAddress).
								Msg("Started standard server without queue support")
						}
					}

					if !enableServer && !enableQueue {
						return fmt.Errorf("at least one of server or queue mode must be enabled")
					}

					sigint := make(chan os.Signal, 1)
					signal.Notify(sigint, os.Interrupt)
					<-sigint
					logging.Logger().Info().Msg("Received sigint, shutting down")

					if len(workers) > 0 {
						logging.Logger().Info().Msg("Stopping queue workers...")
						for i, worker := range workers {
							logging.Logger().Info().Int("worker_id", i+1).Msg("Stopping worker")
							worker.Stop()
						}

						time.Sleep(2 * time.Second)
						logging.Logger().Info().Msg("All queue workers stopped")
					}

					if enableServer {
						logging.Logger().Info().Msg("Stopping HTTP server...")
						instance.RequestStop()
						instance.AwaitStop()
						logging.Logger().Info().Msg("HTTP server stopped")
					}

					if redisQueue != nil {
						if stats, err := redisQueue.GetQueueStats(); err == nil {
							logging.Logger().Info().Interface("final_queue_stats", stats).Msg("Final queue statistics")
						}
					}

					logging.Logger().Info().Msg("Shutdown completed")
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
					&cli.BoolFlag{Name: "address-append", Usage: "Run batch address append circuit", Required: false},
					&cli.StringFlag{Name: "keys-dir", Usage: "Directory where circuit key files are stored", Value: "./proving-keys/", Required: false},
					&cli.StringSliceFlag{Name: "keys-file", Aliases: []string{"k"}, Value: cli.NewStringSlice(), Usage: "Proving system file"},
					&cli.StringSliceFlag{
						Name:  "circuit",
						Usage: "Specify the circuits to enable (inclusion, non-inclusion, combined, append, update, append-test, update-test, address-append, address-append-test)",
						Value: cli.NewStringSlice("inclusion", "non-inclusion", "combined", "append", "update", "append-test", "update-test", "address-append", "address-append-test"),
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

					// TODO: differentiate between address circuits by tree height depending on inputs
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
						logging.Logger().Info().Msgf("Tree Height: %d", treeHeight)
						logging.Logger().Info().Msgf("Compressed Accounts: %d", compressedAccounts)
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
							if provingSystem.TreeHeight == params.Height && provingSystem.BatchSize == params.BatchSize {
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
					} else if context.Bool("address-append") {
						var params prover.BatchAddressAppendParameters
						err = json.Unmarshal(inputsBytes, &params)
						if err != nil {
							return err
						}

						for _, provingSystem := range psv2 {
							if provingSystem.TreeHeight == params.TreeHeight && provingSystem.BatchSize == params.BatchSize {
								proof, err = provingSystem.ProveBatchAddressAppend(&params)
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
						publicInputsHashStr := context.String("publicInputsHash")
						publicInputsHash, err := prover.ParseBigInt(publicInputsHashStr)
						if err != nil {
							return fmt.Errorf("failed to parse roots: %v", err)
						}

						switch circuit {
						case "inclusion":
							verifyErr = s.VerifyInclusion(*publicInputsHash, &proof)
						case "non-inclusion":
							verifyErr = s.VerifyNonInclusion(*publicInputsHash, &proof)
						case "combined":
							verifyErr = s.VerifyCombined(*publicInputsHash, &proof)
						default:
							return fmt.Errorf("invalid circuit type for ProvingSystemV1: %s", circuit)
						}
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
					&cli.UintFlag{Name: "state-tree-height", Usage: "Merkle tree height", Required: true},
					&cli.UintFlag{Name: "address-tree-height", Usage: "Indexed Merkle tree height", Required: true},
					&cli.UintFlag{Name: "compressed-accounts", Usage: "Number of compressed accounts", Required: true},
				},
				Action: func(context *cli.Context) error {
					path := context.String("output")
					stateTreeHeight := uint32(context.Uint("state-tree-height"))
					addressTreeHeight := uint32(context.Uint("address-tree-height"))
					compressedAccounts := uint32(context.Uint("compressed-accounts"))

					logging.Logger().Info().Msg("Extracting gnark circuit to Lean")
					circuitString, err := prover.ExtractLean(stateTreeHeight, addressTreeHeight, compressedAccounts)
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
	runMode := prover.LocalRpc
	switch runModeString {
	case "rpc":
		logging.Logger().Info().Msg("Running in rpc mode")
		runMode = prover.Rpc
	case "local-rpc":
		logging.Logger().Info().Msg("Running in local-rpc mode")
		runMode = prover.LocalRpc
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

func debugProvingSystemKeys(keysDirPath string, runMode prover.RunMode, circuits []string) {
	logging.Logger().Info().
		Str("keysDirPath", keysDirPath).
		Str("runMode", string(runMode)).
		Strs("circuits", circuits).
		Msg("Debug: Loading proving system keys")

	keys := prover.GetKeys(keysDirPath, runMode, circuits)
	for _, key := range keys {
		if _, err := os.Stat(key); err != nil {
			if os.IsNotExist(err) {
				logging.Logger().Error().
					Str("key", key).
					Msg("Key file does not exist")
			} else {
				logging.Logger().Error().
					Str("key", key).
					Err(err).
					Msg("Error checking key file")
			}
		} else {
			fileInfo, err := os.Stat(key)
			if err != nil {
				logging.Logger().Error().
					Str("key", key).
					Err(err).
					Msg("Error getting key file info")
			} else {
				logging.Logger().Info().
					Str("key", key).
					Int64("size", fileInfo.Size()).
					Str("mode", fileInfo.Mode().String()).
					Msg("Key file exists")
			}
		}
	}
}

func startCleanupRoutines(redisQueue *server.RedisQueue) {
	logging.Logger().Info().Msg("Running immediate cleanup on startup")

	if err := redisQueue.CleanupOldRequests(); err != nil {
		logging.Logger().Error().
			Err(err).
			Msg("Failed to cleanup old proof requests on startup")
	} else {
		logging.Logger().Info().Msg("Startup cleanup of old proof requests completed")
	}

	if err := redisQueue.CleanupOldResults(); err != nil {
		logging.Logger().Error().
			Err(err).
			Msg("Failed to cleanup old results on startup")
	} else {
		logging.Logger().Info().Msg("Startup cleanup of old results completed")
	}

	// Start cleanup for old proof requests (every 10 minutes)
	go func() {
		requestTicker := time.NewTicker(10 * time.Minute)
		defer requestTicker.Stop()

		logging.Logger().Info().Msg("Started old proof requests cleanup routine (every 10 minutes)")

		for range requestTicker.C {
			if err := redisQueue.CleanupOldRequests(); err != nil {
				logging.Logger().Error().
					Err(err).
					Msg("Failed to cleanup old proof requests")
			} else {
				logging.Logger().Debug().Msg("Old proof requests cleanup completed")
			}
		}
	}()

	// Start less frequent cleanup for old results (every 1 hour)
	go func() {
		resultTicker := time.NewTicker(1 * time.Hour)
		defer resultTicker.Stop()

		logging.Logger().Info().Msg("Started old results cleanup routine (every 1 hour)")

		for range resultTicker.C {
			if err := redisQueue.CleanupOldResults(); err != nil {
				logging.Logger().Error().
					Err(err).
					Msg("Failed to cleanup old results")
			} else {
				logging.Logger().Debug().Msg("Old results cleanup completed")
			}
		}
	}()
}

// containsCircuit checks if the circuits slice contains the specified circuit
func containsCircuit(circuits []string, circuit string) bool {
	for _, c := range circuits {
		if c == circuit {
			return true
		}
	}
	return false
}
