package main_test

import (
	"fmt"
	"io"
	"light/light-prover/logging"
	"light/light-prover/prover"
	"light/light-prover/server"
	"math/big"
	"net/http"
	"strings"
	"testing"

	gnarkLogger "github.com/consensys/gnark/logger"
)

const ProverAddress = "localhost:8081"
const MetricsAddress = "localhost:9999"

var instance server.RunningJob

func proveEndpoint() string {
	return "http://" + ProverAddress + "/prove"
}

func StartServer() {
	logging.Logger().Info().Msg("Setting up the prover")
	var circuitTypes = []prover.CircuitType{prover.Inclusion, prover.NonInclusion, prover.Combined, prover.BatchAppend, prover.Insertion}
	var keys = prover.GetKeys("./proving-keys/", circuitTypes)
	var pssv1 []*prover.ProvingSystemV1
	var pssv2 []*prover.ProvingSystemV2

	for _, key := range keys {
		system, err := prover.ReadSystemFromFile(key)
		if err != nil {
			logging.Logger().Info().Msgf("Error reading proving system from file: %s", key)
			panic(err)
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

	serverCfg := server.Config{
		ProverAddress:  ProverAddress,
		MetricsAddress: MetricsAddress,
	}
	logging.Logger().Info().Msg("Starting the server")
	instance = server.Run(&serverCfg, pssv1, pssv2)
	logging.Logger().Info().Msg("Running the tests")
}

func StopServer() {
	instance.RequestStop()
	instance.AwaitStop()
}

func TestMain(m *testing.M) {
	gnarkLogger.Set(*logging.Logger())
	StartServer()
	m.Run()
	StopServer()
}

func TestWrongMethod(t *testing.T) {
	response, err := http.Get(proveEndpoint())
	if err != nil {
		t.Fatal(err)
	}
	if response.StatusCode != http.StatusMethodNotAllowed {
		t.Fatalf("Expected status code %d, got %d", http.StatusMethodNotAllowed, response.StatusCode)
	}
}

func TestInclusionHappyPath26_1(t *testing.T) {
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

func TestInclusionHappyPath26_12348(t *testing.T) {
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

func TestInclusionHappyPath26_1_JSON(t *testing.T) {

	testInput := `
{"input-compressed-accounts": [{"root":"0x1ebf5c4eb04bf878b46937be63d12308bb14841813441f041812ea54ecb7b2d5","pathIndex":0,"pathElements":["0x0","0x2098f5fb9e239eab3ceac3f27b81e481dc3124d55ffed523a839ee8446b64864","0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1","0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238","0x7f9d837cb17b0d36320ffe93ba52345f1b728571a568265caac97559dbc952a","0x2b94cf5e8746b3f5c9631f4c5df32907a699c58c94b2ad4d7b5cec1639183f55","0x2dee93c5a666459646ea7d22cca9e1bcfed71e6951b953611d11dda32ea09d78","0x78295e5a22b84e982cf601eb639597b8b0515a88cb5ac7fa8a4aabe3c87349d","0x2fa5e5f18f6027a6501bec864564472a616b2e274a41211a444cbe3a99f3cc61","0xe884376d0d8fd21ecb780389e941f66e45e7acce3e228ab3e2156a614fcd747","0x1b7201da72494f1e28717ad1a52eb469f95892f957713533de6175e5da190af2","0x1f8d8822725e36385200c0b201249819a6e6e1e4650808b5bebc6bface7d7636","0x2c5d82f66c914bafb9701589ba8cfcfb6162b0a12acf88a8d0879a0471b5f85a","0x14c54148a0940bb820957f5adf3fa1134ef5c4aaa113f4646458f270e0bfbfd0","0x190d33b12f986f961e10c0ee44d8b9af11be25588cad89d416118e4bf4ebe80c","0x22f98aa9ce704152ac17354914ad73ed1167ae6596af510aa5b3649325e06c92","0x2a7c7c9b6ce5880b9f6f228d72bf6a575a526f29c66ecceef8b753d38bba7323","0x2e8186e558698ec1c67af9c14d463ffc470043c9c2988b954d75dd643f36b992","0xf57c5571e9a4eab49e2c8cf050dae948aef6ead647392273546249d1c1ff10f","0x1830ee67b5fb554ad5f63d4388800e1cfe78e310697d46e43c9ce36134f72cca","0x2134e76ac5d21aab186c2be1dd8f84ee880a1e46eaf712f9d371b6df22191f3e","0x19df90ec844ebc4ffeebd866f33859b0c051d8c958ee3aa88f8f8df3db91a5b1","0x18cca2a66b5c0787981e69aefd84852d74af0e93ef4912b4648c05f722efe52b","0x2388909415230d1b4d1304d2d54f473a628338f2efad83fadf05644549d2538d","0x27171fb4a97b6cc0e9e8f543b5294de866a2af2c9c8d0b1d96e673e4529ed540","0x2ff6650540f629fd5711a0bc74fc0d28dcb230b9392583e5f8d59696dde6ae21"],"leaf":"0x29176100eaa962bdc1fe6c654d6a3c130e96a4d1168b33848b897dc502820133"}]}	
`
	response, err := http.Post(proveEndpoint(), "application/json", strings.NewReader(testInput))
	if err != nil {
		t.Fatal(err)
	}
	if response.StatusCode != http.StatusOK {
		t.Fatalf("Expected status code %d, got %d", http.StatusOK, response.StatusCode)
	}
}

func TestInclusionWrongInPathIndices(t *testing.T) {
	testInput := `
{"input-compressed-accounts": [{"root":"0x1ebf5c4eb04bf878b46937be63d12308bb14841813441f041812ea54ecb7b2d5","pathIndex":1,"pathElements":["0x0","0x2098f5fb9e239eab3ceac3f27b81e481dc3124d55ffed523a839ee8446b64864","0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1","0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238","0x7f9d837cb17b0d36320ffe93ba52345f1b728571a568265caac97559dbc952a","0x2b94cf5e8746b3f5c9631f4c5df32907a699c58c94b2ad4d7b5cec1639183f55","0x2dee93c5a666459646ea7d22cca9e1bcfed71e6951b953611d11dda32ea09d78","0x78295e5a22b84e982cf601eb639597b8b0515a88cb5ac7fa8a4aabe3c87349d","0x2fa5e5f18f6027a6501bec864564472a616b2e274a41211a444cbe3a99f3cc61","0xe884376d0d8fd21ecb780389e941f66e45e7acce3e228ab3e2156a614fcd747","0x1b7201da72494f1e28717ad1a52eb469f95892f957713533de6175e5da190af2","0x1f8d8822725e36385200c0b201249819a6e6e1e4650808b5bebc6bface7d7636","0x2c5d82f66c914bafb9701589ba8cfcfb6162b0a12acf88a8d0879a0471b5f85a","0x14c54148a0940bb820957f5adf3fa1134ef5c4aaa113f4646458f270e0bfbfd0","0x190d33b12f986f961e10c0ee44d8b9af11be25588cad89d416118e4bf4ebe80c","0x22f98aa9ce704152ac17354914ad73ed1167ae6596af510aa5b3649325e06c92","0x2a7c7c9b6ce5880b9f6f228d72bf6a575a526f29c66ecceef8b753d38bba7323","0x2e8186e558698ec1c67af9c14d463ffc470043c9c2988b954d75dd643f36b992","0xf57c5571e9a4eab49e2c8cf050dae948aef6ead647392273546249d1c1ff10f","0x1830ee67b5fb554ad5f63d4388800e1cfe78e310697d46e43c9ce36134f72cca","0x2134e76ac5d21aab186c2be1dd8f84ee880a1e46eaf712f9d371b6df22191f3e","0x19df90ec844ebc4ffeebd866f33859b0c051d8c958ee3aa88f8f8df3db91a5b1","0x18cca2a66b5c0787981e69aefd84852d74af0e93ef4912b4648c05f722efe52b","0x2388909415230d1b4d1304d2d54f473a628338f2efad83fadf05644549d2538d","0x27171fb4a97b6cc0e9e8f543b5294de866a2af2c9c8d0b1d96e673e4529ed540","0x2ff6650540f629fd5711a0bc74fc0d28dcb230b9392583e5f8d59696dde6ae21"],"leaf":"0x29176100eaa962bdc1fe6c654d6a3c130e96a4d1168b33848b897dc502820133"}]}	
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

func TestInclusionWrongInPathElements(t *testing.T) {
	testInput := `
{"input-compressed-accounts": [{"root":"0x1ebf5c4eb04bf878b46937be63d12308bb14841813441f041812ea54ecb7b2d5","pathIndex":0,"pathElements":["0x1","0x2098f5fb9e239eab3ceac3f27b81e481dc3124d55ffed523a839ee8446b64864","0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1","0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238","0x7f9d837cb17b0d36320ffe93ba52345f1b728571a568265caac97559dbc952a","0x2b94cf5e8746b3f5c9631f4c5df32907a699c58c94b2ad4d7b5cec1639183f55","0x2dee93c5a666459646ea7d22cca9e1bcfed71e6951b953611d11dda32ea09d78","0x78295e5a22b84e982cf601eb639597b8b0515a88cb5ac7fa8a4aabe3c87349d","0x2fa5e5f18f6027a6501bec864564472a616b2e274a41211a444cbe3a99f3cc61","0xe884376d0d8fd21ecb780389e941f66e45e7acce3e228ab3e2156a614fcd747","0x1b7201da72494f1e28717ad1a52eb469f95892f957713533de6175e5da190af2","0x1f8d8822725e36385200c0b201249819a6e6e1e4650808b5bebc6bface7d7636","0x2c5d82f66c914bafb9701589ba8cfcfb6162b0a12acf88a8d0879a0471b5f85a","0x14c54148a0940bb820957f5adf3fa1134ef5c4aaa113f4646458f270e0bfbfd0","0x190d33b12f986f961e10c0ee44d8b9af11be25588cad89d416118e4bf4ebe80c","0x22f98aa9ce704152ac17354914ad73ed1167ae6596af510aa5b3649325e06c92","0x2a7c7c9b6ce5880b9f6f228d72bf6a575a526f29c66ecceef8b753d38bba7323","0x2e8186e558698ec1c67af9c14d463ffc470043c9c2988b954d75dd643f36b992","0xf57c5571e9a4eab49e2c8cf050dae948aef6ead647392273546249d1c1ff10f","0x1830ee67b5fb554ad5f63d4388800e1cfe78e310697d46e43c9ce36134f72cca","0x2134e76ac5d21aab186c2be1dd8f84ee880a1e46eaf712f9d371b6df22191f3e","0x19df90ec844ebc4ffeebd866f33859b0c051d8c958ee3aa88f8f8df3db91a5b1","0x18cca2a66b5c0787981e69aefd84852d74af0e93ef4912b4648c05f722efe52b","0x2388909415230d1b4d1304d2d54f473a628338f2efad83fadf05644549d2538d","0x27171fb4a97b6cc0e9e8f543b5294de866a2af2c9c8d0b1d96e673e4529ed540","0x2ff6650540f629fd5711a0bc74fc0d28dcb230b9392583e5f8d59696dde6ae21"],"leaf":"0x29176100eaa962bdc1fe6c654d6a3c130e96a4d1168b33848b897dc502820133"}]}	
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

func TestInclusionWrongRoot(t *testing.T) {
	testInput := `
{"input-compressed-accounts": [{"root":"0x0","pathIndex":0,"pathElements":["0x0","0x2098f5fb9e239eab3ceac3f27b81e481dc3124d55ffed523a839ee8446b64864","0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1","0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238","0x7f9d837cb17b0d36320ffe93ba52345f1b728571a568265caac97559dbc952a","0x2b94cf5e8746b3f5c9631f4c5df32907a699c58c94b2ad4d7b5cec1639183f55","0x2dee93c5a666459646ea7d22cca9e1bcfed71e6951b953611d11dda32ea09d78","0x78295e5a22b84e982cf601eb639597b8b0515a88cb5ac7fa8a4aabe3c87349d","0x2fa5e5f18f6027a6501bec864564472a616b2e274a41211a444cbe3a99f3cc61","0xe884376d0d8fd21ecb780389e941f66e45e7acce3e228ab3e2156a614fcd747","0x1b7201da72494f1e28717ad1a52eb469f95892f957713533de6175e5da190af2","0x1f8d8822725e36385200c0b201249819a6e6e1e4650808b5bebc6bface7d7636","0x2c5d82f66c914bafb9701589ba8cfcfb6162b0a12acf88a8d0879a0471b5f85a","0x14c54148a0940bb820957f5adf3fa1134ef5c4aaa113f4646458f270e0bfbfd0","0x190d33b12f986f961e10c0ee44d8b9af11be25588cad89d416118e4bf4ebe80c","0x22f98aa9ce704152ac17354914ad73ed1167ae6596af510aa5b3649325e06c92","0x2a7c7c9b6ce5880b9f6f228d72bf6a575a526f29c66ecceef8b753d38bba7323","0x2e8186e558698ec1c67af9c14d463ffc470043c9c2988b954d75dd643f36b992","0xf57c5571e9a4eab49e2c8cf050dae948aef6ead647392273546249d1c1ff10f","0x1830ee67b5fb554ad5f63d4388800e1cfe78e310697d46e43c9ce36134f72cca","0x2134e76ac5d21aab186c2be1dd8f84ee880a1e46eaf712f9d371b6df22191f3e","0x19df90ec844ebc4ffeebd866f33859b0c051d8c958ee3aa88f8f8df3db91a5b1","0x18cca2a66b5c0787981e69aefd84852d74af0e93ef4912b4648c05f722efe52b","0x2388909415230d1b4d1304d2d54f473a628338f2efad83fadf05644549d2538d","0x27171fb4a97b6cc0e9e8f543b5294de866a2af2c9c8d0b1d96e673e4529ed540","0x2ff6650540f629fd5711a0bc74fc0d28dcb230b9392583e5f8d59696dde6ae21"],"leaf":"0x29176100eaa962bdc1fe6c654d6a3c130e96a4d1168b33848b897dc502820133"}]}	
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

func TestParsingEmptyTreeWithOneLeaf(t *testing.T) {
	testInput := `
	{"input-compressed-accounts": [{"root":"0x1ebf5c4eb04bf878b46937be63d12308bb14841813441f041812ea54ecb7b2d5","pathIndex":0,"pathElements":["0x0","0x2098f5fb9e239eab3ceac3f27b81e481dc3124d55ffed523a839ee8446b64864","0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1","0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238","0x7f9d837cb17b0d36320ffe93ba52345f1b728571a568265caac97559dbc952a","0x2b94cf5e8746b3f5c9631f4c5df32907a699c58c94b2ad4d7b5cec1639183f55","0x2dee93c5a666459646ea7d22cca9e1bcfed71e6951b953611d11dda32ea09d78","0x78295e5a22b84e982cf601eb639597b8b0515a88cb5ac7fa8a4aabe3c87349d","0x2fa5e5f18f6027a6501bec864564472a616b2e274a41211a444cbe3a99f3cc61","0xe884376d0d8fd21ecb780389e941f66e45e7acce3e228ab3e2156a614fcd747","0x1b7201da72494f1e28717ad1a52eb469f95892f957713533de6175e5da190af2","0x1f8d8822725e36385200c0b201249819a6e6e1e4650808b5bebc6bface7d7636","0x2c5d82f66c914bafb9701589ba8cfcfb6162b0a12acf88a8d0879a0471b5f85a","0x14c54148a0940bb820957f5adf3fa1134ef5c4aaa113f4646458f270e0bfbfd0","0x190d33b12f986f961e10c0ee44d8b9af11be25588cad89d416118e4bf4ebe80c","0x22f98aa9ce704152ac17354914ad73ed1167ae6596af510aa5b3649325e06c92","0x2a7c7c9b6ce5880b9f6f228d72bf6a575a526f29c66ecceef8b753d38bba7323","0x2e8186e558698ec1c67af9c14d463ffc470043c9c2988b954d75dd643f36b992","0xf57c5571e9a4eab49e2c8cf050dae948aef6ead647392273546249d1c1ff10f","0x1830ee67b5fb554ad5f63d4388800e1cfe78e310697d46e43c9ce36134f72cca","0x2134e76ac5d21aab186c2be1dd8f84ee880a1e46eaf712f9d371b6df22191f3e","0x19df90ec844ebc4ffeebd866f33859b0c051d8c958ee3aa88f8f8df3db91a5b1","0x18cca2a66b5c0787981e69aefd84852d74af0e93ef4912b4648c05f722efe52b","0x2388909415230d1b4d1304d2d54f473a628338f2efad83fadf05644549d2538d","0x27171fb4a97b6cc0e9e8f543b5294de866a2af2c9c8d0b1d96e673e4529ed540","0x2ff6650540f629fd5711a0bc74fc0d28dcb230b9392583e5f8d59696dde6ae21"],"leaf":"0x29176100eaa962bdc1fe6c654d6a3c130e96a4d1168b33848b897dc502820133"}]}
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

func TestNonInclusionHappyPath26_1_JSON(t *testing.T) {
	testInput := `
  {"new-addresses": [
    {
      "root": "0xbfe2d9e57ace69971b010340a2eb1d9f1c9b078c7b9b3c90063b83617a84ef9",
      "value": "0x202",
      "pathIndex": 17,
      "pathElements": [
        "0x0",
        "0x2098f5fb9e239eab3ceac3f27b81e481dc3124d55ffed523a839ee8446b64864",
        "0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1",
        "0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238",
        "0x7f9d837cb17b0d36320ffe93ba52345f1b728571a568265caac97559dbc952a",
        "0x2b94cf5e8746b3f5c9631f4c5df32907a699c58c94b2ad4d7b5cec1639183f55",
        "0x2dee93c5a666459646ea7d22cca9e1bcfed71e6951b953611d11dda32ea09d78",
        "0x78295e5a22b84e982cf601eb639597b8b0515a88cb5ac7fa8a4aabe3c87349d",
        "0x2fa5e5f18f6027a6501bec864564472a616b2e274a41211a444cbe3a99f3cc61",
        "0xe884376d0d8fd21ecb780389e941f66e45e7acce3e228ab3e2156a614fcd747",
        "0x1b7201da72494f1e28717ad1a52eb469f95892f957713533de6175e5da190af2",
        "0x1f8d8822725e36385200c0b201249819a6e6e1e4650808b5bebc6bface7d7636",
        "0x2c5d82f66c914bafb9701589ba8cfcfb6162b0a12acf88a8d0879a0471b5f85a",
        "0x14c54148a0940bb820957f5adf3fa1134ef5c4aaa113f4646458f270e0bfbfd0",
        "0x190d33b12f986f961e10c0ee44d8b9af11be25588cad89d416118e4bf4ebe80c",
        "0x22f98aa9ce704152ac17354914ad73ed1167ae6596af510aa5b3649325e06c92",
        "0x2a7c7c9b6ce5880b9f6f228d72bf6a575a526f29c66ecceef8b753d38bba7323",
        "0x2e8186e558698ec1c67af9c14d463ffc470043c9c2988b954d75dd643f36b992",
        "0xf57c5571e9a4eab49e2c8cf050dae948aef6ead647392273546249d1c1ff10f",
        "0x1830ee67b5fb554ad5f63d4388800e1cfe78e310697d46e43c9ce36134f72cca",
        "0x2134e76ac5d21aab186c2be1dd8f84ee880a1e46eaf712f9d371b6df22191f3e",
        "0x19df90ec844ebc4ffeebd866f33859b0c051d8c958ee3aa88f8f8df3db91a5b1",
        "0x18cca2a66b5c0787981e69aefd84852d74af0e93ef4912b4648c05f722efe52b",
        "0x2388909415230d1b4d1304d2d54f473a628338f2efad83fadf05644549d2538d",
        "0x27171fb4a97b6cc0e9e8f543b5294de866a2af2c9c8d0b1d96e673e4529ed540",
        "0x2ff6650540f629fd5711a0bc74fc0d28dcb230b9392583e5f8d59696dde6ae21"
      ],
      "leafLowerRangeValue": "0x201",
      "leafHigherRangeValue": "0x203",
      "nextIndex": 46336290
    }
  ]
}
`

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

func TestCombinedHappyPath_JSON(t *testing.T) {
	testInput := `
{
  "input-compressed-accounts": [
    {
      "root": "0x1ebf5c4eb04bf878b46937be63d12308bb14841813441f041812ea54ecb7b2d5",
      "pathIndex": 0,
      "pathElements": [
        "0x0",
        "0x2098f5fb9e239eab3ceac3f27b81e481dc3124d55ffed523a839ee8446b64864",
        "0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1",
        "0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238",
        "0x7f9d837cb17b0d36320ffe93ba52345f1b728571a568265caac97559dbc952a",
        "0x2b94cf5e8746b3f5c9631f4c5df32907a699c58c94b2ad4d7b5cec1639183f55",
        "0x2dee93c5a666459646ea7d22cca9e1bcfed71e6951b953611d11dda32ea09d78",
        "0x78295e5a22b84e982cf601eb639597b8b0515a88cb5ac7fa8a4aabe3c87349d",
        "0x2fa5e5f18f6027a6501bec864564472a616b2e274a41211a444cbe3a99f3cc61",
        "0xe884376d0d8fd21ecb780389e941f66e45e7acce3e228ab3e2156a614fcd747",
        "0x1b7201da72494f1e28717ad1a52eb469f95892f957713533de6175e5da190af2",
        "0x1f8d8822725e36385200c0b201249819a6e6e1e4650808b5bebc6bface7d7636",
        "0x2c5d82f66c914bafb9701589ba8cfcfb6162b0a12acf88a8d0879a0471b5f85a",
        "0x14c54148a0940bb820957f5adf3fa1134ef5c4aaa113f4646458f270e0bfbfd0",
        "0x190d33b12f986f961e10c0ee44d8b9af11be25588cad89d416118e4bf4ebe80c",
        "0x22f98aa9ce704152ac17354914ad73ed1167ae6596af510aa5b3649325e06c92",
        "0x2a7c7c9b6ce5880b9f6f228d72bf6a575a526f29c66ecceef8b753d38bba7323",
        "0x2e8186e558698ec1c67af9c14d463ffc470043c9c2988b954d75dd643f36b992",
        "0xf57c5571e9a4eab49e2c8cf050dae948aef6ead647392273546249d1c1ff10f",
        "0x1830ee67b5fb554ad5f63d4388800e1cfe78e310697d46e43c9ce36134f72cca",
        "0x2134e76ac5d21aab186c2be1dd8f84ee880a1e46eaf712f9d371b6df22191f3e",
        "0x19df90ec844ebc4ffeebd866f33859b0c051d8c958ee3aa88f8f8df3db91a5b1",
        "0x18cca2a66b5c0787981e69aefd84852d74af0e93ef4912b4648c05f722efe52b",
        "0x2388909415230d1b4d1304d2d54f473a628338f2efad83fadf05644549d2538d",
        "0x27171fb4a97b6cc0e9e8f543b5294de866a2af2c9c8d0b1d96e673e4529ed540",
        "0x2ff6650540f629fd5711a0bc74fc0d28dcb230b9392583e5f8d59696dde6ae21"
      ],
      "leaf": "0x29176100eaa962bdc1fe6c654d6a3c130e96a4d1168b33848b897dc502820133"
    }
  ],
  "new-addresses": [
    {
      "root": "0xbfe2d9e57ace69971b010340a2eb1d9f1c9b078c7b9b3c90063b83617a84ef9",
      "value": "0x202",
      "pathIndex": 17,
      "pathElements": [
        "0x0",
        "0x2098f5fb9e239eab3ceac3f27b81e481dc3124d55ffed523a839ee8446b64864",
        "0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1",
        "0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238",
        "0x7f9d837cb17b0d36320ffe93ba52345f1b728571a568265caac97559dbc952a",
        "0x2b94cf5e8746b3f5c9631f4c5df32907a699c58c94b2ad4d7b5cec1639183f55",
        "0x2dee93c5a666459646ea7d22cca9e1bcfed71e6951b953611d11dda32ea09d78",
        "0x78295e5a22b84e982cf601eb639597b8b0515a88cb5ac7fa8a4aabe3c87349d",
        "0x2fa5e5f18f6027a6501bec864564472a616b2e274a41211a444cbe3a99f3cc61",
        "0xe884376d0d8fd21ecb780389e941f66e45e7acce3e228ab3e2156a614fcd747",
        "0x1b7201da72494f1e28717ad1a52eb469f95892f957713533de6175e5da190af2",
        "0x1f8d8822725e36385200c0b201249819a6e6e1e4650808b5bebc6bface7d7636",
        "0x2c5d82f66c914bafb9701589ba8cfcfb6162b0a12acf88a8d0879a0471b5f85a",
        "0x14c54148a0940bb820957f5adf3fa1134ef5c4aaa113f4646458f270e0bfbfd0",
        "0x190d33b12f986f961e10c0ee44d8b9af11be25588cad89d416118e4bf4ebe80c",
        "0x22f98aa9ce704152ac17354914ad73ed1167ae6596af510aa5b3649325e06c92",
        "0x2a7c7c9b6ce5880b9f6f228d72bf6a575a526f29c66ecceef8b753d38bba7323",
        "0x2e8186e558698ec1c67af9c14d463ffc470043c9c2988b954d75dd643f36b992",
        "0xf57c5571e9a4eab49e2c8cf050dae948aef6ead647392273546249d1c1ff10f",
        "0x1830ee67b5fb554ad5f63d4388800e1cfe78e310697d46e43c9ce36134f72cca",
        "0x2134e76ac5d21aab186c2be1dd8f84ee880a1e46eaf712f9d371b6df22191f3e",
        "0x19df90ec844ebc4ffeebd866f33859b0c051d8c958ee3aa88f8f8df3db91a5b1",
        "0x18cca2a66b5c0787981e69aefd84852d74af0e93ef4912b4648c05f722efe52b",
        "0x2388909415230d1b4d1304d2d54f473a628338f2efad83fadf05644549d2538d",
        "0x27171fb4a97b6cc0e9e8f543b5294de866a2af2c9c8d0b1d96e673e4529ed540",
        "0x2ff6650540f629fd5711a0bc74fc0d28dcb230b9392583e5f8d59696dde6ae21"
      ],
      "leafLowerRangeValue": "0x201",
      "leafHigherRangeValue": "0x203",
      "nextIndex": 46336290
    }
  ]
}

`
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

func TestBatchAppendHappyPath(t *testing.T) {
	treeDepth := uint32(26)
	batchSize := uint32(1000)
	startIndex := uint32(0)
	params := prover.BuildAndUpdateBatchAppendParameters(treeDepth, batchSize, startIndex, nil)

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

func TestBatchAppendWithPreviousState(t *testing.T) {
	treeDepth := uint32(26)
	batchSize := uint32(100)
	startIndex := uint32(0)

	// First batch
	params1 := prover.BuildAndUpdateBatchAppendParameters(treeDepth, batchSize, startIndex, nil)
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
	params2 := prover.BuildAndUpdateBatchAppendParameters(treeDepth, batchSize, startIndex, &params1)
	jsonBytes2, _ := params2.MarshalJSON()
	response2, err := http.Post(proveEndpoint(), "application/json", bytes.NewBuffer(jsonBytes2))
	if err != nil {
		t.Fatal(err)
	}
	if response2.StatusCode != http.StatusOK {
		t.Fatalf("Second batch: Expected status code %d, got %d", http.StatusOK, response2.StatusCode)
	}
}


func TestInsertionHappyPath26_1(t *testing.T) {
	insertionParams := merkletree.BuildTestInsertionTree(26, 1, false)
	jsonBytes, err := insertionParams.MarshalJSON()
	fmt.Println(string(jsonBytes))
	if err != nil {
		t.Fatal(err)
	}

	response, err := http.Post(proveEndpoint(), "application/json", bytes.NewBuffer(jsonBytes))
	if err != nil {
		t.Fatal(err)

	}
	if response.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(response.Body)
		t.Fatalf("Expected status code %d, got %d. Response body: %s", http.StatusBadRequest, response.StatusCode, string(body))
	}
}

func TestInsertionHappyPath26_8(t *testing.T) {
	insertionParams := merkletree.BuildTestInsertionTree(26, 8, false)
	jsonBytes, err := insertionParams.MarshalJSON()
	if err != nil {
		t.Fatal(err)
	}

	response, err := http.Post(proveEndpoint(), "application/json", bytes.NewBuffer(jsonBytes))
	if err != nil {
		t.Fatal(err)

	}
	if response.StatusCode != http.StatusOK {
		t.Fatalf("Expected status code %d, got %d", http.StatusOK, response.StatusCode)
	}
}

func TestInsertionWrongPreRoot(t *testing.T) {
	insertionParams := merkletree.BuildTestInsertionTree(26, 1, false)

	// Modify the pre-root to be incorrect
	incorrectPreRoot := big.NewInt(0)
	insertionParams.PreRoot = *incorrectPreRoot

	jsonBytes, err := insertionParams.MarshalJSON()
	if err != nil {
		t.Fatal(err)
	}

	response, err := http.Post(proveEndpoint(), "application/json", bytes.NewBuffer(jsonBytes))

	if err != nil {
		t.Fatal(err)
	}

	if response.StatusCode != http.StatusBadRequest {
		body, _ := io.ReadAll(response.Body)
		t.Fatalf("Expected status code %d, got %d. Response body: %s", http.StatusBadRequest, response.StatusCode, string(body))
	}

	responseBody, err := io.ReadAll(response.Body)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(responseBody), "proving_error") {
		t.Fatalf("Expected error message to be tagged with 'proving_error', got %s", string(responseBody))
	}
}

func TestInsertionWrongMerkleProof(t *testing.T) {
	insertionParams := merkletree.BuildTestInsertionTree(26, 1, false)
	jsonBytes, _ := insertionParams.MarshalJSON()
	fmt.Println(string(jsonBytes))
	// Modify the Merkle proof to be incorrect
	for i := range insertionParams.MerkleProofs {
		for j := range insertionParams.MerkleProofs[i] {
			insertionParams.MerkleProofs[i][j] = *big.NewInt(1) // Set all elements to 1 instead of 0
		}
	}

	jsonBytes, err := insertionParams.MarshalJSON()
	if err != nil {
		t.Fatal(err)
	}

	response, err := http.Post(proveEndpoint(), "application/json", bytes.NewBuffer(jsonBytes))
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

func TestParsingInsertionInput(t *testing.T) {
	insertionParams := merkletree.BuildTestInsertionTree(26, 1, false)
	jsonBytes, err := insertionParams.MarshalJSON()
	if err != nil {
		t.Fatal(err)
	}
	fmt.Println(string(jsonBytes))

	var parsedParams prover.InsertionParameters
	err = json.Unmarshal(jsonBytes, &parsedParams)
	if err != nil {
		t.Errorf("error parsing input: %v", err)
	}

	if parsedParams.PreRoot.Cmp(&insertionParams.PreRoot) != 0 {
		t.Errorf("Invalid preRoot: expected %s, got %s", insertionParams.PreRoot.Text(16), parsedParams.PreRoot.Text(16))
	}

	if parsedParams.PostRoot.Cmp(&insertionParams.PostRoot) != 0 {
		t.Errorf("Invalid postRoot: expected %s, got %s", insertionParams.PostRoot.Text(16), parsedParams.PostRoot.Text(16))
	}

	if parsedParams.StartIndex != insertionParams.StartIndex {
		t.Errorf("Invalid startIndex: expected %d, got %d", insertionParams.StartIndex, parsedParams.StartIndex)
	}

	if len(parsedParams.Leaves) != len(insertionParams.Leaves) {
		t.Errorf("Invalid number of leaves: expected %d, got %d", len(insertionParams.Leaves), len(parsedParams.Leaves))
	}

	for i, leaf := range parsedParams.Leaves {
		if leaf.Cmp(&insertionParams.Leaves[i]) != 0 {
			t.Errorf("Invalid leaf at index %d: expected %s, got %s", i, insertionParams.Leaves[i].Text(16), leaf.Text(16))
		}
	}

	if len(parsedParams.MerkleProofs) != len(insertionParams.MerkleProofs) {
		t.Errorf("Invalid number of Merkle proofs: expected %d, got %d", len(insertionParams.MerkleProofs), len(parsedParams.MerkleProofs))
	}

	for i, proof := range parsedParams.MerkleProofs {
		if len(proof) != len(insertionParams.MerkleProofs[i]) {
			t.Errorf("Invalid Merkle proof length at index %d: expected %d, got %d", i, len(insertionParams.MerkleProofs[i]), len(proof))
		}
		for j, element := range proof {
			if element.Cmp(&insertionParams.MerkleProofs[i][j]) != 0 {
				t.Errorf("Invalid Merkle proof element at index [%d][%d]: expected %s, got %s", i, j, insertionParams.MerkleProofs[i][j].Text(16), element.Text(16))
			}
		}
	}
}

func TestBatchUpdateHappyPath26_1(t *testing.T) {
	batchUpdateParams := merkletree.BuildTestBatchUpdateTree(26, 1)
	jsonBytes, err := json.Marshal(batchUpdateParams)
	fmt.Println(string(jsonBytes))
	if err != nil {
		t.Fatal(err)
	}

	response, err := http.Post(proveEndpoint(), "application/json", bytes.NewBuffer(jsonBytes))
	if err != nil {
		t.Fatal(err)
	}
	if response.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(response.Body)
		t.Fatalf("Expected status code %d, got %d. Response body: %s", http.StatusOK, response.StatusCode, string(body))
	}
}

func TestBatchUpdateHappyPath26_8(t *testing.T) {
	batchUpdateParams := merkletree.BuildTestBatchUpdateTree(26, 8)

	fmt.Printf("Debug - BatchUpdateParameters:\n")
	fmt.Printf("PreRoot: %s\n", batchUpdateParams.PreRoot.String())
	fmt.Printf("PostRoot: %s\n", batchUpdateParams.PostRoot.String())
	fmt.Printf("StartIndex: %d\n", batchUpdateParams.StartIndex)
	fmt.Printf("OldLeaves: %v\n", batchUpdateParams.OldLeaves)
	fmt.Printf("NewLeaves: %v\n", batchUpdateParams.NewLeaves)
	fmt.Printf("MerkleProofs length: %d\n", len(batchUpdateParams.MerkleProofs))

	jsonBytes, err := batchUpdateParams.MarshalJSON()
	if err != nil {
		t.Fatal(err)
	}

	response, err := http.Post(proveEndpoint(), "application/json", bytes.NewBuffer(jsonBytes))
	if err != nil {
		t.Fatal(err)
	}
	body, _ := io.ReadAll(response.Body)
	if response.StatusCode != http.StatusOK {
		t.Fatalf("Expected status code %d, got %d: %s", http.StatusOK, response.StatusCode, string(body))
	}
}

func TestBatchUpdateWrongPreRoot(t *testing.T) {
	batchUpdateParams := merkletree.BuildTestBatchUpdateTree(26, 1)

	// Modify the pre-root to be incorrect
	incorrectPreRoot := big.NewInt(0)
	batchUpdateParams.PreRoot = *incorrectPreRoot

	jsonBytes, err := batchUpdateParams.MarshalJSON()
	if err != nil {
		t.Fatal(err)
	}

	response, err := http.Post(proveEndpoint(), "application/json", bytes.NewBuffer(jsonBytes))
	if err != nil {
		t.Fatal(err)
	}

	if response.StatusCode != http.StatusBadRequest {
		body, _ := io.ReadAll(response.Body)
		t.Fatalf("Expected status code %d, got %d. Response body: %s", http.StatusBadRequest, response.StatusCode, string(body))
	}
}

func TestBatchUpdateWrongMerkleProof(t *testing.T) {
	batchUpdateParams := merkletree.BuildTestBatchUpdateTree(26, 1)

	// Modify the Merkle proof to be incorrect
	for i := range batchUpdateParams.MerkleProofs {
		for j := range batchUpdateParams.MerkleProofs[i] {
			batchUpdateParams.MerkleProofs[i][j] = *big.NewInt(1) // Set all elements to 1 instead of the correct value
		}
	}

	jsonBytes, err := batchUpdateParams.MarshalJSON()
	if err != nil {
		t.Fatal(err)
	}

	response, err := http.Post(proveEndpoint(), "application/json", bytes.NewBuffer(jsonBytes))
	if err != nil {
		t.Fatal(err)
	}
	if response.StatusCode != http.StatusBadRequest {
		t.Fatalf("Expected status code %d, got %d", http.StatusBadRequest, response.StatusCode)
	}
}

func TestParsingBatchUpdateInput(t *testing.T) {
	batchUpdateParams := merkletree.BuildTestBatchUpdateTree(26, 1)
	jsonBytes, err := batchUpdateParams.MarshalJSON()
	if err != nil {
		t.Fatal(err)
	}

	var parsedParams prover.BatchUpdateParameters
	err = json.Unmarshal(jsonBytes, &parsedParams)
	if err != nil {
		t.Errorf("error parsing input: %v", err)
	}

	if parsedParams.PreRoot.Cmp(&batchUpdateParams.PreRoot) != 0 {
		t.Errorf("Invalid preRoot: expected %s, got %s", batchUpdateParams.PreRoot.Text(16), parsedParams.PreRoot.Text(16))
	}

	if parsedParams.PostRoot.Cmp(&batchUpdateParams.PostRoot) != 0 {
		t.Errorf("Invalid postRoot: expected %s, got %s", batchUpdateParams.PostRoot.Text(16), parsedParams.PostRoot.Text(16))
	}

	if parsedParams.StartIndex != batchUpdateParams.StartIndex {
		t.Errorf("Invalid startIndex: expected %d, got %d", batchUpdateParams.StartIndex, parsedParams.StartIndex)
	}

	if len(parsedParams.OldLeaves) != len(batchUpdateParams.OldLeaves) {
		t.Errorf("Invalid number of old leaves: expected %d, got %d", len(batchUpdateParams.OldLeaves), len(parsedParams.OldLeaves))
	}

	if len(parsedParams.NewLeaves) != len(batchUpdateParams.NewLeaves) {
		t.Errorf("Invalid number of new leaves: expected %d, got %d", len(batchUpdateParams.NewLeaves), len(parsedParams.NewLeaves))
	}

	for i, leaf := range parsedParams.OldLeaves {
		if leaf.Cmp(&batchUpdateParams.OldLeaves[i]) != 0 {
			t.Errorf("Invalid old leaf at index %d: expected %s, got %s", i, batchUpdateParams.OldLeaves[i].Text(16), leaf.Text(16))
		}
	}

	for i, leaf := range parsedParams.NewLeaves {
		if leaf.Cmp(&batchUpdateParams.NewLeaves[i]) != 0 {
			t.Errorf("Invalid new leaf at index %d: expected %s, got %s", i, batchUpdateParams.NewLeaves[i].Text(16), leaf.Text(16))
		}
	}

	if len(parsedParams.MerkleProofs) != len(batchUpdateParams.MerkleProofs) {
		t.Errorf("Invalid number of Merkle proofs: expected %d, got %d", len(batchUpdateParams.MerkleProofs), len(parsedParams.MerkleProofs))
	}

	for i, proof := range parsedParams.MerkleProofs {
		if len(proof) != len(batchUpdateParams.MerkleProofs[i]) {
			t.Errorf("Invalid Merkle proof length at index %d: expected %d, got %d", i, len(batchUpdateParams.MerkleProofs[i]), len(proof))
		}
		for j, element := range proof {
			if element.Cmp(&batchUpdateParams.MerkleProofs[i][j]) != 0 {
				t.Errorf("Invalid Merkle proof element at index [%d][%d]: expected %s, got %s", i, j, batchUpdateParams.MerkleProofs[i][j].Text(16), element.Text(16))
			}
		}
	}
}