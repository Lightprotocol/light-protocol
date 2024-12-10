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
	if isLightweight {
		keys = prover.GetKeys("./proving-keys/", prover.FullTest, []string{})
	} else {
		keys = prover.GetKeys("./proving-keys/", prover.Full, []string{})
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
	instance = server.Run(&serverCfg, pssv1, pssv2)

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
	t.Run("testInclusionHappyPath26_12348", testInclusionHappyPath26_12348)
	t.Run("testNonInclusionHappyPath26_12348", testNonInclusionHappyPath26_12348)
	t.Run("testInclusionHappyPath26_1_JSON", testInclusionHappyPath26_1_JSON)
	t.Run("testInclusionWrongInPathIndices", testInclusionWrongInPathIndices)
	t.Run("testInclusionWrongInPathElements", testInclusionWrongInPathElements)
	t.Run("testInclusionWrongRoot", testInclusionWrongRoot)
	// t.Run("testParsingEmptyTreeWithOneLeaf", testParsingEmptyTreeWithOneLeaf)
	t.Run("testNonInclusionHappyPath26_1_JSON", testNonInclusionHappyPath26_1_JSON)
	t.Run("testCombinedHappyPath_JSON", testCombinedHappyPath_JSON)
}

// runFullOnlyTests contains tests that should only run in full mode
func runFullOnlyTests(t *testing.T) {
	t.Run("testBatchAppendWithSubtreesHappyPath26_1000", testBatchAppendWithSubtreesHappyPath26_1000)
	t.Run("testBatchAppendWithSubtreesPreviousState26_100", testBatchAppendWithSubtreesPreviousState26_100)

	t.Run("testBatchAppendWithProofsHappyPath26_1000", testBatchAppendWithProofsHappyPath26_1000)
	t.Run("testBatchAppendWithProofsPreviousState26_100", testBatchAppendWithProofsPreviousState26_100)

	t.Run("testBatchUpdateHappyPath26_100", testBatchUpdateHappyPath26_100)
	t.Run("testBatchUpdateHappyPath26_500", testBatchUpdateHappyPath26_500)
	t.Run("testBatchUpdateHappyPath26_1000", testBatchUpdateHappyPath26_1000)

	t.Run("testBatchAddressAppendHappyPath40_100", testBatchAddressAppendHappyPath40_100)
	t.Run("testBatchAddressAppendHappyPath40_500", testBatchAddressAppendHappyPath40_500)
	t.Run("testBatchAddressAppendHappyPath40_250", testBatchAddressAppendHappyPath40_250)
	t.Run("testBatchAddressAppendHappyPath40_1000", testBatchAddressAppendHappyPath40_1000)
	t.Run("testBatchAddressAppendWithPreviousState40_100", testBatchAddressAppendWithPreviousState40_100)
}

func runLightweightOnlyTests(t *testing.T) {
	t.Run("testBatchAppendWithSubtreesHappyPath26_10", testBatchAppendWithSubtreesHappyPath26_10)
	t.Run("testBatchAppendWithSubtreesPreviousState26_10", testBatchAppendWithSubtreesPreviousState26_10)

	t.Run("testBatchAppendWithProofsHappyPath26_10", testBatchAppendWithProofsHappyPath26_10)
	t.Run("testBatchAppendWithProofsPreviousState26_10", testBatchAppendWithProofsPreviousState26_10)

	t.Run("testBatchUpdateHappyPath26_10", testBatchUpdateHappyPath26_10)
	t.Run("testBatchUpdateWithPreviousState26_10", testBatchUpdateWithPreviousState26_10)
	t.Run("testBatchUpdateInvalidInput26_10", testBatchUpdateInvalidInput26_10)

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

func testInclusionHappyPath26_1(t *testing.T) {
	tree := prover.BuildTestTree(26, 1, false)

	// convert tree t to json
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

func testInclusionHappyPath26_12348(t *testing.T) {
	for _, compressedAccounts := range []int{1, 2, 3, 4, 8} {
		tree := prover.BuildTestTree(26, compressedAccounts, false)
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

func testNonInclusionHappyPath26_12348(t *testing.T) {
	for _, compressedAccounts := range []int{1, 2} {
		tree := prover.BuildValidTestNonInclusionTree(26, compressedAccounts, false)
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

func testInclusionHappyPath26_1_JSON(t *testing.T) {

	testInput := `
{"circuitType":"inclusion","stateTreeHeight":26,"publicInputHash":"0x1851430d4ce42cfd16ba8324280a1f420f34ecbd1e05c844bc9f3c570b3ae436","inputCompressedAccounts":[{"root":"0x1ebf5c4eb04bf878b46937be63d12308bb14841813441f041812ea54ecb7b2d5","pathIndex":0,"pathElements":["0x0","0x2098f5fb9e239eab3ceac3f27b81e481dc3124d55ffed523a839ee8446b64864","0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1","0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238","0x7f9d837cb17b0d36320ffe93ba52345f1b728571a568265caac97559dbc952a","0x2b94cf5e8746b3f5c9631f4c5df32907a699c58c94b2ad4d7b5cec1639183f55","0x2dee93c5a666459646ea7d22cca9e1bcfed71e6951b953611d11dda32ea09d78","0x78295e5a22b84e982cf601eb639597b8b0515a88cb5ac7fa8a4aabe3c87349d","0x2fa5e5f18f6027a6501bec864564472a616b2e274a41211a444cbe3a99f3cc61","0xe884376d0d8fd21ecb780389e941f66e45e7acce3e228ab3e2156a614fcd747","0x1b7201da72494f1e28717ad1a52eb469f95892f957713533de6175e5da190af2","0x1f8d8822725e36385200c0b201249819a6e6e1e4650808b5bebc6bface7d7636","0x2c5d82f66c914bafb9701589ba8cfcfb6162b0a12acf88a8d0879a0471b5f85a","0x14c54148a0940bb820957f5adf3fa1134ef5c4aaa113f4646458f270e0bfbfd0","0x190d33b12f986f961e10c0ee44d8b9af11be25588cad89d416118e4bf4ebe80c","0x22f98aa9ce704152ac17354914ad73ed1167ae6596af510aa5b3649325e06c92","0x2a7c7c9b6ce5880b9f6f228d72bf6a575a526f29c66ecceef8b753d38bba7323","0x2e8186e558698ec1c67af9c14d463ffc470043c9c2988b954d75dd643f36b992","0xf57c5571e9a4eab49e2c8cf050dae948aef6ead647392273546249d1c1ff10f","0x1830ee67b5fb554ad5f63d4388800e1cfe78e310697d46e43c9ce36134f72cca","0x2134e76ac5d21aab186c2be1dd8f84ee880a1e46eaf712f9d371b6df22191f3e","0x19df90ec844ebc4ffeebd866f33859b0c051d8c958ee3aa88f8f8df3db91a5b1","0x18cca2a66b5c0787981e69aefd84852d74af0e93ef4912b4648c05f722efe52b","0x2388909415230d1b4d1304d2d54f473a628338f2efad83fadf05644549d2538d","0x27171fb4a97b6cc0e9e8f543b5294de866a2af2c9c8d0b1d96e673e4529ed540","0x2ff6650540f629fd5711a0bc74fc0d28dcb230b9392583e5f8d59696dde6ae21"],"leaf":"0x29176100eaa962bdc1fe6c654d6a3c130e96a4d1168b33848b897dc502820133"}]}
`
	response, err := http.Post(proveEndpoint(), "application/json", strings.NewReader(testInput))
	if err != nil {
		t.Fatal(err)
	}
	if response.StatusCode != http.StatusOK {
		t.Fatalf("Expected status code %d, got %d", http.StatusOK, response.StatusCode)
	}
}

func testInclusionWrongInPathIndices(t *testing.T) {
	testInput := `
{"circuitType":"inclusion","stateTreeHeight":26,"publicInputHash":"0x1851430d4ce42cfd16ba8324280a1f420f34ecbd1e05c844bc9f3c570b3ae436","inputCompressedAccounts":[{"root":"0x1ebf5c4eb04bf878b46937be63d12308bb14841813441f041812ea54ecb7b2d5","pathIndex":1,"pathElements":["0x0","0x2098f5fb9e239eab3ceac3f27b81e481dc3124d55ffed523a839ee8446b64864","0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1","0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238","0x7f9d837cb17b0d36320ffe93ba52345f1b728571a568265caac97559dbc952a","0x2b94cf5e8746b3f5c9631f4c5df32907a699c58c94b2ad4d7b5cec1639183f55","0x2dee93c5a666459646ea7d22cca9e1bcfed71e6951b953611d11dda32ea09d78","0x78295e5a22b84e982cf601eb639597b8b0515a88cb5ac7fa8a4aabe3c87349d","0x2fa5e5f18f6027a6501bec864564472a616b2e274a41211a444cbe3a99f3cc61","0xe884376d0d8fd21ecb780389e941f66e45e7acce3e228ab3e2156a614fcd747","0x1b7201da72494f1e28717ad1a52eb469f95892f957713533de6175e5da190af2","0x1f8d8822725e36385200c0b201249819a6e6e1e4650808b5bebc6bface7d7636","0x2c5d82f66c914bafb9701589ba8cfcfb6162b0a12acf88a8d0879a0471b5f85a","0x14c54148a0940bb820957f5adf3fa1134ef5c4aaa113f4646458f270e0bfbfd0","0x190d33b12f986f961e10c0ee44d8b9af11be25588cad89d416118e4bf4ebe80c","0x22f98aa9ce704152ac17354914ad73ed1167ae6596af510aa5b3649325e06c92","0x2a7c7c9b6ce5880b9f6f228d72bf6a575a526f29c66ecceef8b753d38bba7323","0x2e8186e558698ec1c67af9c14d463ffc470043c9c2988b954d75dd643f36b992","0xf57c5571e9a4eab49e2c8cf050dae948aef6ead647392273546249d1c1ff10f","0x1830ee67b5fb554ad5f63d4388800e1cfe78e310697d46e43c9ce36134f72cca","0x2134e76ac5d21aab186c2be1dd8f84ee880a1e46eaf712f9d371b6df22191f3e","0x19df90ec844ebc4ffeebd866f33859b0c051d8c958ee3aa88f8f8df3db91a5b1","0x18cca2a66b5c0787981e69aefd84852d74af0e93ef4912b4648c05f722efe52b","0x2388909415230d1b4d1304d2d54f473a628338f2efad83fadf05644549d2538d","0x27171fb4a97b6cc0e9e8f543b5294de866a2af2c9c8d0b1d96e673e4529ed540","0x2ff6650540f629fd5711a0bc74fc0d28dcb230b9392583e5f8d59696dde6ae21"],"leaf":"0x29176100eaa962bdc1fe6c654d6a3c130e96a4d1168b33848b897dc502820133"}]}
	`
	response, err := http.Post(proveEndpoint(), "application/json", strings.NewReader(testInput))
	if err != nil {
		t.Fatal(err)
	}
	if response.StatusCode != http.StatusBadRequest {
		t.Fatalf("Expected status code %d, got %d", http.StatusBadRequest, response.StatusCode)
	}

	responseBody, err := io.ReadAll(response.Body)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(responseBody), "proving_error") {
		t.Fatalf("Expected error message to be tagged with 'proving_error', got %s", string(responseBody))
	}
}

func testInclusionWrongInPathElements(t *testing.T) {
	testInput := `
{"circuitType":"inclusion","stateTreeHeight":26,"publicInputHash":"0x1851430d4ce42cfd16ba8324280a1f420f34ecbd1e05c844bc9f3c570b3ae436","inputCompressedAccounts":[{"root":"0x1ebf5c4eb04bf878b46937be63d12308bb14841813441f041812ea54ecb7b2d5","pathIndex":0,"pathElements":["0x1","0x2098f5fb9e239eab3ceac3f27b81e481dc3124d55ffed523a839ee8446b64864","0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1","0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238","0x7f9d837cb17b0d36320ffe93ba52345f1b728571a568265caac97559dbc952a","0x2b94cf5e8746b3f5c9631f4c5df32907a699c58c94b2ad4d7b5cec1639183f55","0x2dee93c5a666459646ea7d22cca9e1bcfed71e6951b953611d11dda32ea09d78","0x78295e5a22b84e982cf601eb639597b8b0515a88cb5ac7fa8a4aabe3c87349d","0x2fa5e5f18f6027a6501bec864564472a616b2e274a41211a444cbe3a99f3cc61","0xe884376d0d8fd21ecb780389e941f66e45e7acce3e228ab3e2156a614fcd747","0x1b7201da72494f1e28717ad1a52eb469f95892f957713533de6175e5da190af2","0x1f8d8822725e36385200c0b201249819a6e6e1e4650808b5bebc6bface7d7636","0x2c5d82f66c914bafb9701589ba8cfcfb6162b0a12acf88a8d0879a0471b5f85a","0x14c54148a0940bb820957f5adf3fa1134ef5c4aaa113f4646458f270e0bfbfd0","0x190d33b12f986f961e10c0ee44d8b9af11be25588cad89d416118e4bf4ebe80c","0x22f98aa9ce704152ac17354914ad73ed1167ae6596af510aa5b3649325e06c92","0x2a7c7c9b6ce5880b9f6f228d72bf6a575a526f29c66ecceef8b753d38bba7323","0x2e8186e558698ec1c67af9c14d463ffc470043c9c2988b954d75dd643f36b992","0xf57c5571e9a4eab49e2c8cf050dae948aef6ead647392273546249d1c1ff10f","0x1830ee67b5fb554ad5f63d4388800e1cfe78e310697d46e43c9ce36134f72cca","0x2134e76ac5d21aab186c2be1dd8f84ee880a1e46eaf712f9d371b6df22191f3e","0x19df90ec844ebc4ffeebd866f33859b0c051d8c958ee3aa88f8f8df3db91a5b1","0x18cca2a66b5c0787981e69aefd84852d74af0e93ef4912b4648c05f722efe52b","0x2388909415230d1b4d1304d2d54f473a628338f2efad83fadf05644549d2538d","0x27171fb4a97b6cc0e9e8f543b5294de866a2af2c9c8d0b1d96e673e4529ed540","0x2ff6650540f629fd5711a0bc74fc0d28dcb230b9392583e5f8d59696dde6ae21"],"leaf":"0x29176100eaa962bdc1fe6c654d6a3c130e96a4d1168b33848b897dc502820133"}]}
`
	response, err := http.Post(proveEndpoint(), "application/json", strings.NewReader(testInput))
	if err != nil {
		t.Fatal(err)
	}
	if response.StatusCode != http.StatusBadRequest {
		t.Fatalf("Expected status code %d, got %d", http.StatusBadRequest, response.StatusCode)
	}

	responseBody, err := io.ReadAll(response.Body)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(responseBody), "proving_error") {
		t.Fatalf("Expected error message to be tagged with 'proving_error', got %s", string(responseBody))
	}
}

func testInclusionWrongRoot(t *testing.T) {
	testInput := `
{"circuitType":"inclusion","stateTreeHeight":26,"publicInputHash":"0x1851430d4ce42cfd16ba8324280a1f420f34ecbd1e05c844bc9f3c570b3ae436","inputCompressedAccounts":[{"root":"0x0","pathIndex":0,"pathElements":["0x0","0x2098f5fb9e239eab3ceac3f27b81e481dc3124d55ffed523a839ee8446b64864","0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1","0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238","0x7f9d837cb17b0d36320ffe93ba52345f1b728571a568265caac97559dbc952a","0x2b94cf5e8746b3f5c9631f4c5df32907a699c58c94b2ad4d7b5cec1639183f55","0x2dee93c5a666459646ea7d22cca9e1bcfed71e6951b953611d11dda32ea09d78","0x78295e5a22b84e982cf601eb639597b8b0515a88cb5ac7fa8a4aabe3c87349d","0x2fa5e5f18f6027a6501bec864564472a616b2e274a41211a444cbe3a99f3cc61","0xe884376d0d8fd21ecb780389e941f66e45e7acce3e228ab3e2156a614fcd747","0x1b7201da72494f1e28717ad1a52eb469f95892f957713533de6175e5da190af2","0x1f8d8822725e36385200c0b201249819a6e6e1e4650808b5bebc6bface7d7636","0x2c5d82f66c914bafb9701589ba8cfcfb6162b0a12acf88a8d0879a0471b5f85a","0x14c54148a0940bb820957f5adf3fa1134ef5c4aaa113f4646458f270e0bfbfd0","0x190d33b12f986f961e10c0ee44d8b9af11be25588cad89d416118e4bf4ebe80c","0x22f98aa9ce704152ac17354914ad73ed1167ae6596af510aa5b3649325e06c92","0x2a7c7c9b6ce5880b9f6f228d72bf6a575a526f29c66ecceef8b753d38bba7323","0x2e8186e558698ec1c67af9c14d463ffc470043c9c2988b954d75dd643f36b992","0xf57c5571e9a4eab49e2c8cf050dae948aef6ead647392273546249d1c1ff10f","0x1830ee67b5fb554ad5f63d4388800e1cfe78e310697d46e43c9ce36134f72cca","0x2134e76ac5d21aab186c2be1dd8f84ee880a1e46eaf712f9d371b6df22191f3e","0x19df90ec844ebc4ffeebd866f33859b0c051d8c958ee3aa88f8f8df3db91a5b1","0x18cca2a66b5c0787981e69aefd84852d74af0e93ef4912b4648c05f722efe52b","0x2388909415230d1b4d1304d2d54f473a628338f2efad83fadf05644549d2538d","0x27171fb4a97b6cc0e9e8f543b5294de866a2af2c9c8d0b1d96e673e4529ed540","0x2ff6650540f629fd5711a0bc74fc0d28dcb230b9392583e5f8d59696dde6ae21"],"leaf":"0x29176100eaa962bdc1fe6c654d6a3c130e96a4d1168b33848b897dc502820133"}]}
`

	response, err := http.Post(proveEndpoint(), "application/json", strings.NewReader(testInput))
	if err != nil {
		t.Fatal(err)
	}

	if response.StatusCode != http.StatusBadRequest {
		t.Fatalf("Expected status code %d, got %d", http.StatusBadRequest, response.StatusCode)
	}
	responseBody, err := io.ReadAll(response.Body)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(responseBody), "proving_error") {
		t.Fatalf("Expected error message to be tagged with 'proving_error', got %s", string(responseBody))
	}
}

func testParsingEmptyTreeWithOneLeaf(t *testing.T) {
	testInput := `
	{"inputCompressedAccounts": [{"root":"0x1ebf5c4eb04bf878b46937be63d12308bb14841813441f041812ea54ecb7b2d5","pathIndex":0,"pathElements":["0x0","0x2098f5fb9e239eab3ceac3f27b81e481dc3124d55ffed523a839ee8446b64864","0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1","0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238","0x7f9d837cb17b0d36320ffe93ba52345f1b728571a568265caac97559dbc952a","0x2b94cf5e8746b3f5c9631f4c5df32907a699c58c94b2ad4d7b5cec1639183f55","0x2dee93c5a666459646ea7d22cca9e1bcfed71e6951b953611d11dda32ea09d78","0x78295e5a22b84e982cf601eb639597b8b0515a88cb5ac7fa8a4aabe3c87349d","0x2fa5e5f18f6027a6501bec864564472a616b2e274a41211a444cbe3a99f3cc61","0xe884376d0d8fd21ecb780389e941f66e45e7acce3e228ab3e2156a614fcd747","0x1b7201da72494f1e28717ad1a52eb469f95892f957713533de6175e5da190af2","0x1f8d8822725e36385200c0b201249819a6e6e1e4650808b5bebc6bface7d7636","0x2c5d82f66c914bafb9701589ba8cfcfb6162b0a12acf88a8d0879a0471b5f85a","0x14c54148a0940bb820957f5adf3fa1134ef5c4aaa113f4646458f270e0bfbfd0","0x190d33b12f986f961e10c0ee44d8b9af11be25588cad89d416118e4bf4ebe80c","0x22f98aa9ce704152ac17354914ad73ed1167ae6596af510aa5b3649325e06c92","0x2a7c7c9b6ce5880b9f6f228d72bf6a575a526f29c66ecceef8b753d38bba7323","0x2e8186e558698ec1c67af9c14d463ffc470043c9c2988b954d75dd643f36b992","0xf57c5571e9a4eab49e2c8cf050dae948aef6ead647392273546249d1c1ff10f","0x1830ee67b5fb554ad5f63d4388800e1cfe78e310697d46e43c9ce36134f72cca","0x2134e76ac5d21aab186c2be1dd8f84ee880a1e46eaf712f9d371b6df22191f3e","0x19df90ec844ebc4ffeebd866f33859b0c051d8c958ee3aa88f8f8df3db91a5b1","0x18cca2a66b5c0787981e69aefd84852d74af0e93ef4912b4648c05f722efe52b","0x2388909415230d1b4d1304d2d54f473a628338f2efad83fadf05644549d2538d","0x27171fb4a97b6cc0e9e8f543b5294de866a2af2c9c8d0b1d96e673e4529ed540","0x2ff6650540f629fd5711a0bc74fc0d28dcb230b9392583e5f8d59696dde6ae21"],"leaf":"0x29176100eaa962bdc1fe6c654d6a3c130e96a4d1168b33848b897dc502820133"}]}
	`

	proofData, err := prover.ParseInput(testInput)
	if err != nil {
		t.Errorf("error parsing input: %v", err)
	}

	tree := prover.BuildTestTree(26, 1, false)

	if len(tree.Inputs) != len(proofData.Inputs) {
		t.Errorf("Invalid shape: expected %d, got %d", len(tree.Inputs), len(proofData.Inputs))
	}

	for i, bi := range tree.Inputs {
		if bi.Root.String() != proofData.Inputs[i].Root.String() {
			t.Errorf("Invalid root: expected %s, got %s", bi.Root.String(), proofData.Inputs[i].Root.String())
		}
		if bi.Leaf.String() != proofData.Inputs[i].Leaf.String() {
			t.Errorf("Invalid leaf: expected %s, got %s", bi.Leaf.String(), proofData.Inputs[i].Leaf.String())
		}

		if bi.PathIndex != proofData.Inputs[i].PathIndex {
			t.Errorf("Invalid pathIndex: expected %d, got %d", bi.PathIndex, proofData.Inputs[i].PathIndex)
		}

		for j, bj := range bi.PathElements {
			if bj.String() != proofData.Inputs[i].PathElements[j].String() {
				t.Errorf("Invalid pathElements: expected %s, got %s", bj.String(), proofData.Inputs[i].PathElements[j].String())
			}
		}
	}
}

func testNonInclusionHappyPath26_1_JSON(t *testing.T) {
	testInput := `{"circuitType":"non-inclusion","addressTreeHeight":26,"publicInputHash":"0x1cbe696806e19049f6af3571ff3782f755437de7c5192c40658b4d55dced9cac","newAddresses":[{"root":"0x1e033b84fe586aa089fc99e4ed7cf839e4f0ac856e234c69d781cb8f49f0eb9b","value":"0x1e0","pathIndex":5,"pathElements":["0x0","0x2098f5fb9e239eab3ceac3f27b81e481dc3124d55ffed523a839ee8446b64864","0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1","0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238","0x7f9d837cb17b0d36320ffe93ba52345f1b728571a568265caac97559dbc952a","0x2b94cf5e8746b3f5c9631f4c5df32907a699c58c94b2ad4d7b5cec1639183f55","0x2dee93c5a666459646ea7d22cca9e1bcfed71e6951b953611d11dda32ea09d78","0x78295e5a22b84e982cf601eb639597b8b0515a88cb5ac7fa8a4aabe3c87349d","0x2fa5e5f18f6027a6501bec864564472a616b2e274a41211a444cbe3a99f3cc61","0xe884376d0d8fd21ecb780389e941f66e45e7acce3e228ab3e2156a614fcd747","0x1b7201da72494f1e28717ad1a52eb469f95892f957713533de6175e5da190af2","0x1f8d8822725e36385200c0b201249819a6e6e1e4650808b5bebc6bface7d7636","0x2c5d82f66c914bafb9701589ba8cfcfb6162b0a12acf88a8d0879a0471b5f85a","0x14c54148a0940bb820957f5adf3fa1134ef5c4aaa113f4646458f270e0bfbfd0","0x190d33b12f986f961e10c0ee44d8b9af11be25588cad89d416118e4bf4ebe80c","0x22f98aa9ce704152ac17354914ad73ed1167ae6596af510aa5b3649325e06c92","0x2a7c7c9b6ce5880b9f6f228d72bf6a575a526f29c66ecceef8b753d38bba7323","0x2e8186e558698ec1c67af9c14d463ffc470043c9c2988b954d75dd643f36b992","0xf57c5571e9a4eab49e2c8cf050dae948aef6ead647392273546249d1c1ff10f","0x1830ee67b5fb554ad5f63d4388800e1cfe78e310697d46e43c9ce36134f72cca","0x2134e76ac5d21aab186c2be1dd8f84ee880a1e46eaf712f9d371b6df22191f3e","0x19df90ec844ebc4ffeebd866f33859b0c051d8c958ee3aa88f8f8df3db91a5b1","0x18cca2a66b5c0787981e69aefd84852d74af0e93ef4912b4648c05f722efe52b","0x2388909415230d1b4d1304d2d54f473a628338f2efad83fadf05644549d2538d","0x27171fb4a97b6cc0e9e8f543b5294de866a2af2c9c8d0b1d96e673e4529ed540","0x2ff6650540f629fd5711a0bc74fc0d28dcb230b9392583e5f8d59696dde6ae21"],"leafLowerRangeValue":"0x1df","leafHigherRangeValue":"0x1e1","nextIndex":42057957}]}`
	response, err := http.Post(proveEndpoint(), "application/json", strings.NewReader(testInput))
	if err != nil {
		t.Fatal(err)
	}

	responseBody, err := io.ReadAll(response.Body)
	if err != nil {
		t.Fatal(err)
	}

	if response.StatusCode != http.StatusOK {
		t.Fatalf("Expected status code %d, got %d %s", http.StatusOK, response.StatusCode, string(responseBody))
	}
}

func testCombinedHappyPath_JSON(t *testing.T) {
	testInput := `{"circuitType":"combined","stateTreeHeight":26,"addressTreeHeight":26,"publicInputHash":"0x244bd1975a3ccf5efe62c73dbe9f238c3fbddb32da8241361bfeec8ac4727d02","inputCompressedAccounts":[{"root":"0x1ebf5c4eb04bf878b46937be63d12308bb14841813441f041812ea54ecb7b2d5","pathIndex":0,"pathElements":["0x0","0x2098f5fb9e239eab3ceac3f27b81e481dc3124d55ffed523a839ee8446b64864","0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1","0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238","0x7f9d837cb17b0d36320ffe93ba52345f1b728571a568265caac97559dbc952a","0x2b94cf5e8746b3f5c9631f4c5df32907a699c58c94b2ad4d7b5cec1639183f55","0x2dee93c5a666459646ea7d22cca9e1bcfed71e6951b953611d11dda32ea09d78","0x78295e5a22b84e982cf601eb639597b8b0515a88cb5ac7fa8a4aabe3c87349d","0x2fa5e5f18f6027a6501bec864564472a616b2e274a41211a444cbe3a99f3cc61","0xe884376d0d8fd21ecb780389e941f66e45e7acce3e228ab3e2156a614fcd747","0x1b7201da72494f1e28717ad1a52eb469f95892f957713533de6175e5da190af2","0x1f8d8822725e36385200c0b201249819a6e6e1e4650808b5bebc6bface7d7636","0x2c5d82f66c914bafb9701589ba8cfcfb6162b0a12acf88a8d0879a0471b5f85a","0x14c54148a0940bb820957f5adf3fa1134ef5c4aaa113f4646458f270e0bfbfd0","0x190d33b12f986f961e10c0ee44d8b9af11be25588cad89d416118e4bf4ebe80c","0x22f98aa9ce704152ac17354914ad73ed1167ae6596af510aa5b3649325e06c92","0x2a7c7c9b6ce5880b9f6f228d72bf6a575a526f29c66ecceef8b753d38bba7323","0x2e8186e558698ec1c67af9c14d463ffc470043c9c2988b954d75dd643f36b992","0xf57c5571e9a4eab49e2c8cf050dae948aef6ead647392273546249d1c1ff10f","0x1830ee67b5fb554ad5f63d4388800e1cfe78e310697d46e43c9ce36134f72cca","0x2134e76ac5d21aab186c2be1dd8f84ee880a1e46eaf712f9d371b6df22191f3e","0x19df90ec844ebc4ffeebd866f33859b0c051d8c958ee3aa88f8f8df3db91a5b1","0x18cca2a66b5c0787981e69aefd84852d74af0e93ef4912b4648c05f722efe52b","0x2388909415230d1b4d1304d2d54f473a628338f2efad83fadf05644549d2538d","0x27171fb4a97b6cc0e9e8f543b5294de866a2af2c9c8d0b1d96e673e4529ed540","0x2ff6650540f629fd5711a0bc74fc0d28dcb230b9392583e5f8d59696dde6ae21"],"leaf":"0x29176100eaa962bdc1fe6c654d6a3c130e96a4d1168b33848b897dc502820133"}],"newAddresses":[{"root":"0x11d8beb2ead34f2db1004c65a9ff1ac015bb62867574704e9416ed13d5c0c7b5","value":"0x9d","pathIndex":18,"pathElements":["0x0","0x2098f5fb9e239eab3ceac3f27b81e481dc3124d55ffed523a839ee8446b64864","0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1","0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238","0x7f9d837cb17b0d36320ffe93ba52345f1b728571a568265caac97559dbc952a","0x2b94cf5e8746b3f5c9631f4c5df32907a699c58c94b2ad4d7b5cec1639183f55","0x2dee93c5a666459646ea7d22cca9e1bcfed71e6951b953611d11dda32ea09d78","0x78295e5a22b84e982cf601eb639597b8b0515a88cb5ac7fa8a4aabe3c87349d","0x2fa5e5f18f6027a6501bec864564472a616b2e274a41211a444cbe3a99f3cc61","0xe884376d0d8fd21ecb780389e941f66e45e7acce3e228ab3e2156a614fcd747","0x1b7201da72494f1e28717ad1a52eb469f95892f957713533de6175e5da190af2","0x1f8d8822725e36385200c0b201249819a6e6e1e4650808b5bebc6bface7d7636","0x2c5d82f66c914bafb9701589ba8cfcfb6162b0a12acf88a8d0879a0471b5f85a","0x14c54148a0940bb820957f5adf3fa1134ef5c4aaa113f4646458f270e0bfbfd0","0x190d33b12f986f961e10c0ee44d8b9af11be25588cad89d416118e4bf4ebe80c","0x22f98aa9ce704152ac17354914ad73ed1167ae6596af510aa5b3649325e06c92","0x2a7c7c9b6ce5880b9f6f228d72bf6a575a526f29c66ecceef8b753d38bba7323","0x2e8186e558698ec1c67af9c14d463ffc470043c9c2988b954d75dd643f36b992","0xf57c5571e9a4eab49e2c8cf050dae948aef6ead647392273546249d1c1ff10f","0x1830ee67b5fb554ad5f63d4388800e1cfe78e310697d46e43c9ce36134f72cca","0x2134e76ac5d21aab186c2be1dd8f84ee880a1e46eaf712f9d371b6df22191f3e","0x19df90ec844ebc4ffeebd866f33859b0c051d8c958ee3aa88f8f8df3db91a5b1","0x18cca2a66b5c0787981e69aefd84852d74af0e93ef4912b4648c05f722efe52b","0x2388909415230d1b4d1304d2d54f473a628338f2efad83fadf05644549d2538d","0x27171fb4a97b6cc0e9e8f543b5294de866a2af2c9c8d0b1d96e673e4529ed540","0x2ff6650540f629fd5711a0bc74fc0d28dcb230b9392583e5f8d59696dde6ae21"],"leafLowerRangeValue":"0x9c","leafHigherRangeValue":"0x9e","nextIndex":53376793}]}`
	response, err := http.Post(proveEndpoint(), "application/json", strings.NewReader(testInput))
	if err != nil {
		t.Fatal(err)
	}

	responseBody, err := io.ReadAll(response.Body)
	if err != nil {
		t.Fatal(err)
	}

	if response.StatusCode != http.StatusOK {
		t.Fatalf("Expected status code %d, got %d %s", http.StatusOK, response.StatusCode, string(responseBody))
	}
}

func testBatchAppendWithSubtreesHappyPath26_1000(t *testing.T) {
	treeDepth := uint32(26)
	batchSize := uint32(1000)
	startIndex := uint32(0)
	params := prover.BuildAndUpdateBatchAppendWithSubtreesParameters(treeDepth, batchSize, startIndex, nil)

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

func testBatchAppendWithProofsHappyPath26_1000(t *testing.T) {
	treeDepth := 26
	batchSize := 1000
	startIndex := 0
	params := prover.BuildTestBatchAppendWithProofsTree(treeDepth, batchSize, nil, startIndex, true)

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

func testBatchAppendWithSubtreesHappyPath26_10(t *testing.T) {
	treeDepth := uint32(26)
	batchSize := uint32(10)
	startIndex := uint32(0)
	params := prover.BuildAndUpdateBatchAppendWithSubtreesParameters(treeDepth, batchSize, startIndex, nil)

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

func testBatchAppendWithProofsHappyPath26_10(t *testing.T) {
	treeDepth := 26
	batchSize := 10
	startIndex := 0
	params := prover.BuildTestBatchAppendWithProofsTree(treeDepth, batchSize, nil, startIndex, true)

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

func testBatchAppendWithSubtreesPreviousState26_100(t *testing.T) {
	treeDepth := uint32(26)
	batchSize := uint32(100)
	startIndex := uint32(0)

	// First batch
	params1 := prover.BuildAndUpdateBatchAppendWithSubtreesParameters(treeDepth, batchSize, startIndex, nil)
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
	params2 := prover.BuildAndUpdateBatchAppendWithSubtreesParameters(treeDepth, batchSize, startIndex, &params1)
	jsonBytes2, _ := params2.MarshalJSON()
	response2, err := http.Post(proveEndpoint(), "application/json", bytes.NewBuffer(jsonBytes2))
	if err != nil {
		t.Fatal(err)
	}
	if response2.StatusCode != http.StatusOK {
		t.Fatalf("Second batch: Expected status code %d, got %d", http.StatusOK, response2.StatusCode)
	}
}

func testBatchAppendWithSubtreesPreviousState26_10(t *testing.T) {
	treeDepth := uint32(26)
	batchSize := uint32(10)
	startIndex := uint32(0)

	// First batch
	params1 := prover.BuildAndUpdateBatchAppendWithSubtreesParameters(treeDepth, batchSize, startIndex, nil)
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
	params2 := prover.BuildAndUpdateBatchAppendWithSubtreesParameters(treeDepth, batchSize, startIndex, &params1)
	jsonBytes2, _ := params2.MarshalJSON()
	response2, err := http.Post(proveEndpoint(), "application/json", bytes.NewBuffer(jsonBytes2))
	if err != nil {
		t.Fatal(err)
	}
	if response2.StatusCode != http.StatusOK {
		t.Fatalf("Second batch: Expected status code %d, got %d", http.StatusOK, response2.StatusCode)
	}
}

func testBatchAppendWithProofsPreviousState26_100(t *testing.T) {
	treeDepth := 26
	batchSize := 100
	startIndex := 0

	// First batch
	params1 := prover.BuildTestBatchAppendWithProofsTree(treeDepth, batchSize, nil, startIndex, true)
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
	params2 := prover.BuildTestBatchAppendWithProofsTree(treeDepth, batchSize, params1.Tree, startIndex, true)
	jsonBytes2, _ := params2.MarshalJSON()
	response2, err := http.Post(proveEndpoint(), "application/json", bytes.NewBuffer(jsonBytes2))
	if err != nil {
		t.Fatal(err)
	}
	if response2.StatusCode != http.StatusOK {
		t.Fatalf("Second batch: Expected status code %d, got %d", http.StatusOK, response2.StatusCode)
	}
}

func testBatchAppendWithProofsPreviousState26_10(t *testing.T) {
	treeDepth := 26
	batchSize := 10
	startIndex := 0

	// First batch
	params1 := prover.BuildTestBatchAppendWithProofsTree(treeDepth, batchSize, nil, startIndex, true)
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
	params2 := prover.BuildTestBatchAppendWithProofsTree(treeDepth, batchSize, params1.Tree, startIndex, true)
	jsonBytes2, _ := params2.MarshalJSON()
	response2, err := http.Post(proveEndpoint(), "application/json", bytes.NewBuffer(jsonBytes2))
	if err != nil {
		t.Fatal(err)
	}
	if response2.StatusCode != http.StatusOK {
		t.Fatalf("Second batch: Expected status code %d, got %d", http.StatusOK, response2.StatusCode)
	}
}

func testBatchUpdateWithPreviousState26_10(t *testing.T) {
	treeDepth := uint32(26)
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

func testBatchUpdateInvalidInput26_10(t *testing.T) {
	treeDepth := uint32(26)
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

func testBatchUpdateHappyPath26_10(t *testing.T) {
	runBatchUpdateTest(t, 26, 10)
}

func testBatchUpdateHappyPath26_100(t *testing.T) {
	runBatchUpdateTest(t, 26, 100)
}

func testBatchUpdateHappyPath26_500(t *testing.T) {
	runBatchUpdateTest(t, 26, 500)
}

func testBatchUpdateHappyPath26_1000(t *testing.T) {
	runBatchUpdateTest(t, 26, 1000)
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
	params, err := prover.BuildTestAddressTree(treeHeight, batchSize, nil, 2)
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
	startIndex := uint32(2)
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

	startIndex += batchSize
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
	startIndex := uint32(0)

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
