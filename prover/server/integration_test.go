package main_test

import (
	"bytes"
	"io"
	"light/light-prover/logging"
	"light/light-prover/prover"
	"light/light-prover/server"
	"math/big"
	"net/http"
	"os"
	"strings"
	"testing"
	"time"

	gnarkLogger "github.com/consensys/gnark/logger"
)

var isLightweightMode bool

const ProverAddress = "localhost:8081"
const MetricsAddress = "localhost:9999"

var instance server.RunningJob

func proveEndpoint() string {
	return "http://" + ProverAddress + "/prove"
}

func StartServer(isLightweight bool) {
	logging.Logger().Info().Msg("Setting up the prover")
	var keys []string
	var runMode prover.RunMode
	if isLightweight {
		keys = prover.GetKeys("./proving-keys/", prover.FullTest, []string{})
		runMode = prover.FullTest
	} else {
		keys = prover.GetKeys("./proving-keys/", prover.Full, []string{})
		runMode = prover.Full
	}
	var pssv1 []*prover.ProvingSystemV1
	var pssv2 []*prover.ProvingSystemV2

	missingKeys := []string{}

	for _, key := range keys {
		system, err := prover.ReadSystemFromFile(key)
		if err != nil {
			if os.IsNotExist(err) {
				logging.Logger().Warn().Msgf("Key file not found: %s. Skipping this key.", key)
				missingKeys = append(missingKeys, key)
				continue
			}
			logging.Logger().Error().Msgf("Error reading proving system from file: %s. Error: %v", key, err)
			continue
		}

		switch s := system.(type) {
		case *prover.ProvingSystemV1:
			pssv1 = append(pssv1, s)
		case *prover.ProvingSystemV2:
			pssv2 = append(pssv2, s)
		default:
			logging.Logger().Info().Msgf("Unknown proving system type for file: %s", key)
			panic("Unknown proving system type")
		}
	}

	if len(missingKeys) > 0 {
		logging.Logger().Warn().Msgf("Some key files are missing. To download %s keys, run: ./scripts/download_keys.sh %s",
			map[bool]string{true: "lightweight", false: "full"}[isLightweight],
			map[bool]string{true: "lightweight", false: "full"}[isLightweight])
	}

	if len(pssv1) == 0 && len(pssv2) == 0 {
		logging.Logger().Fatal().Msg("No valid proving systems found. Cannot start the server. Please ensure you have downloaded the necessary key files.")
		return
	}

	serverCfg := server.Config{
		ProverAddress:  ProverAddress,
		MetricsAddress: MetricsAddress,
	}
	logging.Logger().Info().Msg("Starting the server")
	instance = server.Run(&serverCfg, []string{}, runMode, pssv1, pssv2)

	// sleep for 1 sec to ensure that the server is up and running before running the tests
	time.Sleep(1 * time.Second)

	logging.Logger().Info().Msg("Running the tests")
}

func StopServer() {
	instance.RequestStop()
	instance.AwaitStop()
}

func TestMain(m *testing.M) {
	gnarkLogger.Set(*logging.Logger())
	isLightweightMode = true
	for _, arg := range os.Args {
		if arg == "-test.run=TestFull" {
			isLightweightMode = false
			break
		}
	}

	if isLightweightMode {
		logging.Logger().Info().Msg("Running in lightweight mode")
		logging.Logger().Info().Msg("If you encounter missing key errors, run: ./scripts/download_keys.sh light")
	} else {
		logging.Logger().Info().Msg("Running in full mode")
		logging.Logger().Info().Msg("If you encounter missing key errors, run: ./scripts/download_keys.sh full")
	}

	StartServer(isLightweightMode)
	m.Run()
	StopServer()
}

func TestLightweight(t *testing.T) {
	if !isLightweightMode {
		t.Skip("This test only runs in lightweight mode")
	}
	runCommonTests(t)
	runLightweightOnlyTests(t)
}

func TestFull(t *testing.T) {
	if isLightweightMode {
		t.Skip("This test only runs in full mode")
	}
	runCommonTests(t)
	runFullOnlyTests(t)
}

// runCommonTests contains all tests that should run in both modes
func runCommonTests(t *testing.T) {
	t.Run("testWrongMethod", testWrongMethod)
	t.Run("testInclusionHappyPath32_12348", testInclusionHappyPath32_12348)
	t.Run("testNonInclusionHappyPath40_12348", testNonInclusionHappyPath40_12348)
}

// runFullOnlyTests contains tests that should only run in full mode
func runFullOnlyTests(t *testing.T) {
	t.Run("testBatchAppendHappyPath32_1000", testBatchAppendHappyPath32_1000)
	t.Run("testBatchAppendPreviousState32_100", testBatchAppendPreviousState32_100)

	t.Run("testBatchUpdateHappyPath32_100", testBatchUpdateHappyPath32_100)
	t.Run("testBatchUpdateHappyPath32_500", testBatchUpdateHappyPath32_500)
	t.Run("testBatchUpdateHappyPath32_1000", testBatchUpdateHappyPath32_1000)

	t.Run("testBatchAddressAppendHappyPath40_100", testBatchAddressAppendHappyPath40_100)
	t.Run("testBatchAddressAppendHappyPath40_500", testBatchAddressAppendHappyPath40_500)
	t.Run("testBatchAddressAppendHappyPath40_250", testBatchAddressAppendHappyPath40_250)
	t.Run("testBatchAddressAppendHappyPath40_1000", testBatchAddressAppendHappyPath40_1000)
	t.Run("testBatchAddressAppendWithPreviousState40_100", testBatchAddressAppendWithPreviousState40_100)
}

func runLightweightOnlyTests(t *testing.T) {
	t.Run("testBatchAppendHappyPath32_10", testBatchAppendHappyPath32_10)
	t.Run("testBatchAppendPreviousState32_10", testBatchAppendPreviousState32_10)

	t.Run("testBatchUpdateHappyPath32_10", testBatchUpdateHappyPath32_10)
	t.Run("testBatchUpdateWithPreviousState32_10", testBatchUpdateWithPreviousState32_10)
	t.Run("testBatchUpdateInvalidInput32_10", testBatchUpdateInvalidInput32_10)
	t.Run("testBatchUpdateHappyPath32_10", testBatchUpdateHappyPath32_10)

	t.Run("testBatchAddressAppendHappyPath40_10", testBatchAddressAppendHappyPath40_10)
	t.Run("testBatchAddressAppendWithPreviousState40_10", testBatchAddressAppendWithPreviousState40_10)
	t.Run("testBatchAddressAppendInvalidInput40_10", testBatchAddressAppendInvalidInput40_10)
}

func testWrongMethod(t *testing.T) {
	response, err := http.Get(proveEndpoint())
	if err != nil {
		t.Fatal(err)
	}
	if response.StatusCode != http.StatusMethodNotAllowed {
		t.Fatalf("Expected status code %d, got %d", http.StatusMethodNotAllowed, response.StatusCode)
	}
}

func testInclusionHappyPath32_12348(t *testing.T) {
	for _, compressedAccounts := range []int{1, 2, 3, 4, 8} {
		tree := prover.BuildTestTree(32, compressedAccounts, false)
		jsonBytes, _ := tree.MarshalJSON()
		jsonString := string(jsonBytes)

		response, err := http.Post(proveEndpoint(), "application/json", strings.NewReader(jsonString))
		if err != nil {
			t.Fatal(err)
		}
		if response.StatusCode != http.StatusOK {
			t.Fatalf("Expected status code %d, got %d", http.StatusOK, response.StatusCode)
		}
	}
}

func testNonInclusionHappyPath40_12348(t *testing.T) {
	for _, compressedAccounts := range []int{1, 2} {
		tree := prover.BuildValidTestNonInclusionTree(40, compressedAccounts, false)
		jsonBytes, _ := tree.MarshalJSON()
		jsonString := string(jsonBytes)

		response, err := http.Post(proveEndpoint(), "application/json", strings.NewReader(jsonString))
		if err != nil {
			t.Fatal(err)
		}
		if response.StatusCode != http.StatusOK {
			t.Fatalf("Expected status code %d, got %d", http.StatusOK, response.StatusCode)
		}
	}
}

func testBatchAppendHappyPath32_1000(t *testing.T) {
	treeDepth := 32
	batchSize := 1000
	startIndex := 0
	params := prover.BuildTestBatchAppendTree(treeDepth, batchSize, nil, startIndex, true)

	jsonBytes, _ := params.MarshalJSON()

	response, err := http.Post(proveEndpoint(), "application/json", bytes.NewBuffer(jsonBytes))
	if err != nil {
		t.Fatal(err)
	}
	defer response.Body.Close()

	if response.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(response.Body)
		t.Fatalf("Expected status code %d, got %d. Response body: %s", http.StatusOK, response.StatusCode, string(body))
	}
}

func testBatchAppendHappyPath32_10(t *testing.T) {
	treeDepth := 32
	batchSize := 10
	startIndex := 0
	params := prover.BuildTestBatchAppendTree(treeDepth, batchSize, nil, startIndex, true)

	jsonBytes, _ := params.MarshalJSON()

	response, err := http.Post(proveEndpoint(), "application/json", bytes.NewBuffer(jsonBytes))
	if err != nil {
		t.Fatal(err)
	}
	defer response.Body.Close()

	if response.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(response.Body)
		t.Fatalf("Expected status code %d, got %d. Response body: %s", http.StatusOK, response.StatusCode, string(body))
	}
}

func testBatchAppendPreviousState32_100(t *testing.T) {
	treeDepth := 32
	batchSize := 100
	startIndex := 0

	// First batch
	params1 := prover.BuildTestBatchAppendTree(treeDepth, batchSize, nil, startIndex, true)
	jsonBytes1, _ := params1.MarshalJSON()
	response1, err := http.Post(proveEndpoint(), "application/json", bytes.NewBuffer(jsonBytes1))
	if err != nil {
		t.Fatal(err)
	}
	if response1.StatusCode != http.StatusOK {
		t.Fatalf("First batch: Expected status code %d, got %d", http.StatusOK, response1.StatusCode)
	}

	// Second batch
	startIndex += batchSize
	params2 := prover.BuildTestBatchAppendTree(treeDepth, batchSize, params1.Tree, startIndex, true)
	jsonBytes2, _ := params2.MarshalJSON()
	response2, err := http.Post(proveEndpoint(), "application/json", bytes.NewBuffer(jsonBytes2))
	if err != nil {
		t.Fatal(err)
	}
	if response2.StatusCode != http.StatusOK {
		t.Fatalf("Second batch: Expected status code %d, got %d", http.StatusOK, response2.StatusCode)
	}
}

func testBatchAppendPreviousState32_10(t *testing.T) {
	treeDepth := 32
	batchSize := 10
	startIndex := 0

	// First batch
	params1 := prover.BuildTestBatchAppendTree(treeDepth, batchSize, nil, startIndex, true)
	jsonBytes1, _ := params1.MarshalJSON()
	response1, err := http.Post(proveEndpoint(), "application/json", bytes.NewBuffer(jsonBytes1))
	if err != nil {
		t.Fatal(err)
	}
	if response1.StatusCode != http.StatusOK {
		t.Fatalf("First batch: Expected status code %d, got %d", http.StatusOK, response1.StatusCode)
	}

	// Second batch
	startIndex += batchSize
	params2 := prover.BuildTestBatchAppendTree(treeDepth, batchSize, params1.Tree, startIndex, true)
	jsonBytes2, _ := params2.MarshalJSON()
	response2, err := http.Post(proveEndpoint(), "application/json", bytes.NewBuffer(jsonBytes2))
	if err != nil {
		t.Fatal(err)
	}
	if response2.StatusCode != http.StatusOK {
		t.Fatalf("Second batch: Expected status code %d, got %d", http.StatusOK, response2.StatusCode)
	}
}

func testBatchUpdateWithPreviousState32_10(t *testing.T) {
	treeDepth := uint32(32)
	batchSize := uint32(10)

	// First batch
	params1 := prover.BuildTestBatchUpdateTree(int(treeDepth), int(batchSize), nil, nil)
	jsonBytes1, _ := params1.MarshalJSON()
	response1, err := http.Post(proveEndpoint(), "application/json", bytes.NewBuffer(jsonBytes1))
	if err != nil {
		t.Fatal(err)
	}
	if response1.StatusCode != http.StatusOK {
		t.Fatalf("First batch: Expected status code %d, got %d", http.StatusOK, response1.StatusCode)
	}

	// Second batch
	params2 := prover.BuildTestBatchUpdateTree(int(treeDepth), int(batchSize), params1.Tree, nil)
	jsonBytes2, _ := params2.MarshalJSON()
	response2, err := http.Post(proveEndpoint(), "application/json", bytes.NewBuffer(jsonBytes2))
	if err != nil {
		t.Fatal(err)
	}
	if response2.StatusCode != http.StatusOK {
		t.Fatalf("Second batch: Expected status code %d, got %d", http.StatusOK, response2.StatusCode)
	}

	// Verify that the new root is different from the old root
	if params2.OldRoot.Cmp(params2.NewRoot) == 0 {
		t.Errorf("Expected new root to be different from old root")
	}
}

func testBatchUpdateInvalidInput32_10(t *testing.T) {
	treeDepth := uint32(32)
	batchSize := uint32(10)
	params := prover.BuildTestBatchUpdateTree(int(treeDepth), int(batchSize), nil, nil)

	// Invalidate the input by changing the old root
	params.OldRoot = big.NewInt(0)
	jsonBytes, _ := params.MarshalJSON()

	response, err := http.Post(proveEndpoint(), "application/json", bytes.NewBuffer(jsonBytes))
	if err != nil {
		t.Fatal(err)
	}
	defer response.Body.Close()

	if response.StatusCode != http.StatusBadRequest {
		t.Fatalf("Expected status code %d, got %d", http.StatusBadRequest, response.StatusCode)
	}

	body, _ := io.ReadAll(response.Body)
	if !strings.Contains(string(body), "proving_error") {
		t.Fatalf("Expected error message to contain 'proving_error', got: %s", string(body))
	}
}

func testBatchUpdateHappyPath32_10(t *testing.T) {
	runBatchUpdateTest(t, 32, 10)
}

func testBatchUpdateHappyPath32_100(t *testing.T) {
	runBatchUpdateTest(t, 32, 100)
}

func testBatchUpdateHappyPath32_500(t *testing.T) {
	runBatchUpdateTest(t, 32, 500)
}

func testBatchUpdateHappyPath32_1000(t *testing.T) {
	runBatchUpdateTest(t, 32, 1000)
}

func runBatchUpdateTest(t *testing.T, treeDepth uint32, batchSize uint32) {
	params := prover.BuildTestBatchUpdateTree(int(treeDepth), int(batchSize), nil, nil)

	jsonBytes, err := params.MarshalJSON()
	if err != nil {
		t.Fatalf("Failed to marshal JSON: %v", err)
	}

	response, err := http.Post(proveEndpoint(), "application/json", bytes.NewBuffer(jsonBytes))
	if err != nil {
		t.Fatalf("Failed to send POST request: %v", err)
	}
	defer response.Body.Close()

	if response.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(response.Body)
		t.Fatalf("Expected status code %d, got %d. Response body: %s", http.StatusOK, response.StatusCode, string(body))
	}

	if params.OldRoot.Cmp(params.NewRoot) == 0 {
		t.Errorf("Expected new root to be different from old root")
	}

	t.Logf("Successfully ran batch update test with tree depth %d and batch size %d", treeDepth, batchSize)
}

func testBatchAddressAppendHappyPath40_10(t *testing.T) {
	runBatchAddressAppendTest(t, 40, 10)
}

func testBatchAddressAppendHappyPath40_100(t *testing.T) {
	runBatchAddressAppendTest(t, 40, 100)
}

func testBatchAddressAppendHappyPath40_500(t *testing.T) {
	runBatchAddressAppendTest(t, 40, 500)
}

func testBatchAddressAppendHappyPath40_250(t *testing.T) {
	runBatchAddressAppendTest(t, 40, 250)
}

func testBatchAddressAppendHappyPath40_1000(t *testing.T) {
	runBatchAddressAppendTest(t, 40, 1000)
}

func runBatchAddressAppendTest(t *testing.T, treeHeight uint32, batchSize uint32) {
	params, err := prover.BuildTestAddressTree(treeHeight, batchSize, nil, 1)
	if err != nil {
		t.Fatalf("Failed to build test tree: %v", err)
	}

	jsonBytes, err := params.MarshalJSON()
	if err != nil {
		t.Fatalf("Failed to marshal JSON: %v", err)
	}
	response, err := http.Post(proveEndpoint(), "application/json", bytes.NewBuffer(jsonBytes))
	if err != nil {
		t.Fatalf("Failed to send POST request: %v", err)
	}
	defer response.Body.Close()

	if response.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(response.Body)
		t.Fatalf("Expected status code %d, got %d. Response body: %s", http.StatusOK, response.StatusCode, string(body))
	}

	// Verify that the new root is different from the old root
	if params.OldRoot.Cmp(params.NewRoot) == 0 {
		t.Errorf("Expected new root to be different from old root")
	}

	t.Logf("Successfully ran batch address append test with tree height %d and batch size %d", treeHeight, batchSize)
}

func testBatchAddressAppendWithPreviousState40_10(t *testing.T) {
	runBatchAddressAppendWithPreviousStateTest(t, 40, 10)
}

func testBatchAddressAppendWithPreviousState40_100(t *testing.T) {
	runBatchAddressAppendWithPreviousStateTest(t, 40, 100)
}

func runBatchAddressAppendWithPreviousStateTest(t *testing.T, treeHeight uint32, batchSize uint32) {
	startIndex := uint64(1)
	params1, err := prover.BuildTestAddressTree(treeHeight, batchSize, nil, startIndex)
	if err != nil {
		t.Fatalf("Failed to build first test tree: %v", err)
	}

	jsonBytes1, err := params1.MarshalJSON()
	if err != nil {
		t.Fatalf("Failed to marshal first JSON: %v", err)
	}

	response1, err := http.Post(proveEndpoint(), "application/json", bytes.NewBuffer(jsonBytes1))
	if err != nil {
		t.Fatalf("Failed to send first POST request: %v", err)
	}
	if response1.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(response1.Body)
		t.Fatalf("First batch: Expected status code %d, got %d. Response body: %s",
			http.StatusOK, response1.StatusCode, string(body))
	}
	response1.Body.Close()

	startIndex += uint64(batchSize)
	params2, err := prover.BuildTestAddressTree(treeHeight, batchSize, params1.Tree, startIndex)
	if err != nil {
		t.Fatalf("Failed to build second test tree: %v", err)
	}
	params2.OldRoot = params1.NewRoot

	jsonBytes2, err := params2.MarshalJSON()
	if err != nil {
		t.Fatalf("Failed to marshal second JSON: %v", err)
	}

	response2, err := http.Post(proveEndpoint(), "application/json", bytes.NewBuffer(jsonBytes2))
	if err != nil {
		t.Fatalf("Failed to send second POST request: %v", err)
	}
	if response2.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(response2.Body)
		t.Fatalf("Second batch: Expected status code %d, got %d. Response body: %s",
			http.StatusOK, response2.StatusCode, string(body))
	}
	response2.Body.Close()

	if params2.OldRoot.Cmp(params2.NewRoot) == 0 {
		t.Errorf("Expected new root to be different from old root in second batch")
	}

	t.Logf("Successfully ran batch address append with previous state test with tree height %d and batch size %d",
		treeHeight, batchSize)
}

func testBatchAddressAppendInvalidInput40_10(t *testing.T) {
	treeHeight := uint32(40)
	batchSize := uint32(10)
	startIndex := uint64(0)

	params, err := prover.BuildTestAddressTree(treeHeight, batchSize, nil, startIndex)
	if err != nil {
		t.Fatalf("Failed to build test tree: %v", err)
	}

	// Invalidate input by setting wrong old root
	params.OldRoot = big.NewInt(0)

	jsonBytes, err := params.MarshalJSON()
	if err != nil {
		t.Fatalf("Failed to marshal JSON: %v", err)
	}

	response, err := http.Post(proveEndpoint(), "application/json", bytes.NewBuffer(jsonBytes))
	if err != nil {
		t.Fatalf("Failed to send POST request: %v", err)
	}
	defer response.Body.Close()

	if response.StatusCode != http.StatusBadRequest {
		t.Fatalf("Expected status code %d, got %d", http.StatusBadRequest, response.StatusCode)
	}

	body, _ := io.ReadAll(response.Body)
	if !strings.Contains(string(body), "proving_error") {
		t.Fatalf("Expected error message to contain 'proving_error', got: %s", string(body))
	}

	t.Logf("Successfully ran invalid input test with tree height %d and batch size %d",
		treeHeight, batchSize)
}
