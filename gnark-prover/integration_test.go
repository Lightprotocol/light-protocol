package main_test

import (
	"fmt"
	"io"
	"light/light-prover/config"
	"light/light-prover/logging"
	merkletree "light/light-prover/merkle-tree"
	"light/light-prover/prover"
	"light/light-prover/server"
	"net/http"
	"strings"
	"testing"

	gnarkLogger "github.com/consensys/gnark/logger"
	"github.com/pelletier/go-toml/v2"
)

const ProverAddress = "localhost:8081"
const MetricsAddress = "localhost:9999"

var instance server.RunningJob

func StartServer(cfg config.Config) {
	logging.Logger().Info().Msg("Setting up the prover")

	fmt.Println("Starting test server")
	fmt.Println("Config: ", cfg)

	var pss = make([]*prover.ProvingSystem, len(cfg.Keys))

	for i, key := range cfg.Keys {
		// Another way to instantiate the circuit: prover.SetupInclusion(Depth, NumberOfUtxos)
		// But we need to know the tree depth and the number of UTXOs
		ps, err := prover.ReadSystemFromFile(key)
		if err != nil {
			panic(err)
		}

		pss[i] = ps
	}

	serverCfg := server.Config{
		ProverAddress:  ProverAddress,
		MetricsAddress: MetricsAddress,
	}
	logging.Logger().Info().Msg("Starting the server")
	instance = server.Run(&serverCfg, pss)
	logging.Logger().Info().Msg("Running the tests")
}

func StopServer() {
	instance.RequestStop()
	instance.AwaitStop()
}

func TestMain(m *testing.M) {
	gnarkLogger.Set(*logging.Logger())
	cfg, error := config.ReadConfig("config.toml")
	if error != nil {
		panic("Failed to read config")
	}
	StartServer(cfg)
	m.Run()
	StopServer()
}

func TestConfigFile(t *testing.T) {

	doc := `
keys = ["circuit_26_1", "circuit_26_2", "circuit_26_3", "circuit_26_4", "circuit_26_8"]
`

	var cfg config.Config
	err := toml.Unmarshal([]byte(doc), &cfg)
	if err != nil {
		panic(err)
	}
}

func TestWrongMethod(t *testing.T) {
	response, err := http.Get("http://" + ProverAddress + "/inclusion")
	if err != nil {
		t.Fatal(err)
	}
	if response.StatusCode != http.StatusMethodNotAllowed {
		t.Fatalf("Expected status code %d, got %d", http.StatusMethodNotAllowed, response.StatusCode)
	}
}

func TestInclusionHappyPath26_1(t *testing.T) {
	tree := merkletree.BuildTestTree(26, 1, false)

	// convert tree t to json
	jsonBytes, _ := tree.MarshalJSON()
	jsonString := string(jsonBytes)
	response, err := http.Post("http://"+ProverAddress+"/inclusion", "application/json", strings.NewReader(jsonString))

	if err != nil {
		t.Fatal(err)
	}
	if response.StatusCode != http.StatusOK {
		t.Fatalf("Expected status code %d, got %d", http.StatusOK, response.StatusCode)
	}
}

func TestInclusionHappyPath26_12348(t *testing.T) {
	// create array with values [1,2,3,4,8], iterate over it and print value
	for _, utxos := range []int{1, 2, 3, 4, 8} {
		tree := merkletree.BuildTestTree(26, utxos, false)
		// convert tree t to json
		jsonBytes, _ := tree.MarshalJSON()
		jsonString := string(jsonBytes)

		response, err := http.Post("http://"+ProverAddress+"/inclusion", "application/json", strings.NewReader(jsonString))
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
{"root":["0x1ebf5c4eb04bf878b46937be63d12308bb14841813441f041812ea54ecb7b2d5"],"inPathIndices":[0],"inPathElements":[["0x0","0x2098f5fb9e239eab3ceac3f27b81e481dc3124d55ffed523a839ee8446b64864","0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1","0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238","0x7f9d837cb17b0d36320ffe93ba52345f1b728571a568265caac97559dbc952a","0x2b94cf5e8746b3f5c9631f4c5df32907a699c58c94b2ad4d7b5cec1639183f55","0x2dee93c5a666459646ea7d22cca9e1bcfed71e6951b953611d11dda32ea09d78","0x78295e5a22b84e982cf601eb639597b8b0515a88cb5ac7fa8a4aabe3c87349d","0x2fa5e5f18f6027a6501bec864564472a616b2e274a41211a444cbe3a99f3cc61","0xe884376d0d8fd21ecb780389e941f66e45e7acce3e228ab3e2156a614fcd747","0x1b7201da72494f1e28717ad1a52eb469f95892f957713533de6175e5da190af2","0x1f8d8822725e36385200c0b201249819a6e6e1e4650808b5bebc6bface7d7636","0x2c5d82f66c914bafb9701589ba8cfcfb6162b0a12acf88a8d0879a0471b5f85a","0x14c54148a0940bb820957f5adf3fa1134ef5c4aaa113f4646458f270e0bfbfd0","0x190d33b12f986f961e10c0ee44d8b9af11be25588cad89d416118e4bf4ebe80c","0x22f98aa9ce704152ac17354914ad73ed1167ae6596af510aa5b3649325e06c92","0x2a7c7c9b6ce5880b9f6f228d72bf6a575a526f29c66ecceef8b753d38bba7323","0x2e8186e558698ec1c67af9c14d463ffc470043c9c2988b954d75dd643f36b992","0xf57c5571e9a4eab49e2c8cf050dae948aef6ead647392273546249d1c1ff10f","0x1830ee67b5fb554ad5f63d4388800e1cfe78e310697d46e43c9ce36134f72cca","0x2134e76ac5d21aab186c2be1dd8f84ee880a1e46eaf712f9d371b6df22191f3e","0x19df90ec844ebc4ffeebd866f33859b0c051d8c958ee3aa88f8f8df3db91a5b1","0x18cca2a66b5c0787981e69aefd84852d74af0e93ef4912b4648c05f722efe52b","0x2388909415230d1b4d1304d2d54f473a628338f2efad83fadf05644549d2538d","0x27171fb4a97b6cc0e9e8f543b5294de866a2af2c9c8d0b1d96e673e4529ed540","0x2ff6650540f629fd5711a0bc74fc0d28dcb230b9392583e5f8d59696dde6ae21"]],"leaf":["0x29176100eaa962bdc1fe6c654d6a3c130e96a4d1168b33848b897dc502820133"]}	
`
	response, err := http.Post("http://"+ProverAddress+"/inclusion", "application/json", strings.NewReader(testInput))
	if err != nil {
		t.Fatal(err)
	}
	if response.StatusCode != http.StatusOK {
		t.Fatalf("Expected status code %d, got %d", http.StatusOK, response.StatusCode)
	}
}

func TestInclusionWrongInPathIndices(t *testing.T) {
	testInput := `
	{"root":["0x1ebf5c4eb04bf878b46937be63d12308bb14841813441f041812ea54ecb7b2d5"],"inPathIndices":[1],"inPathElements":[["0x0","0x2098f5fb9e239eab3ceac3f27b81e481dc3124d55ffed523a839ee8446b64864","0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1","0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238","0x7f9d837cb17b0d36320ffe93ba52345f1b728571a568265caac97559dbc952a","0x2b94cf5e8746b3f5c9631f4c5df32907a699c58c94b2ad4d7b5cec1639183f55","0x2dee93c5a666459646ea7d22cca9e1bcfed71e6951b953611d11dda32ea09d78","0x78295e5a22b84e982cf601eb639597b8b0515a88cb5ac7fa8a4aabe3c87349d","0x2fa5e5f18f6027a6501bec864564472a616b2e274a41211a444cbe3a99f3cc61","0xe884376d0d8fd21ecb780389e941f66e45e7acce3e228ab3e2156a614fcd747","0x1b7201da72494f1e28717ad1a52eb469f95892f957713533de6175e5da190af2","0x1f8d8822725e36385200c0b201249819a6e6e1e4650808b5bebc6bface7d7636","0x2c5d82f66c914bafb9701589ba8cfcfb6162b0a12acf88a8d0879a0471b5f85a","0x14c54148a0940bb820957f5adf3fa1134ef5c4aaa113f4646458f270e0bfbfd0","0x190d33b12f986f961e10c0ee44d8b9af11be25588cad89d416118e4bf4ebe80c","0x22f98aa9ce704152ac17354914ad73ed1167ae6596af510aa5b3649325e06c92","0x2a7c7c9b6ce5880b9f6f228d72bf6a575a526f29c66ecceef8b753d38bba7323","0x2e8186e558698ec1c67af9c14d463ffc470043c9c2988b954d75dd643f36b992","0xf57c5571e9a4eab49e2c8cf050dae948aef6ead647392273546249d1c1ff10f","0x1830ee67b5fb554ad5f63d4388800e1cfe78e310697d46e43c9ce36134f72cca","0x2134e76ac5d21aab186c2be1dd8f84ee880a1e46eaf712f9d371b6df22191f3e","0x19df90ec844ebc4ffeebd866f33859b0c051d8c958ee3aa88f8f8df3db91a5b1","0x18cca2a66b5c0787981e69aefd84852d74af0e93ef4912b4648c05f722efe52b","0x2388909415230d1b4d1304d2d54f473a628338f2efad83fadf05644549d2538d","0x27171fb4a97b6cc0e9e8f543b5294de866a2af2c9c8d0b1d96e673e4529ed540","0x2ff6650540f629fd5711a0bc74fc0d28dcb230b9392583e5f8d59696dde6ae21"]],"leaf":["0x29176100eaa962bdc1fe6c654d6a3c130e96a4d1168b33848b897dc502820133"]}	
	`
	response, err := http.Post("http://"+ProverAddress+"/inclusion", "application/json", strings.NewReader(testInput))
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
{"root":["0x1ebf5c4eb04bf878b46937be63d12308bb14841813441f041812ea54ecb7b2d5"],"inPathIndices":[0],"inPathElements":[["0x1","0x2098f5fb9e239eab3ceac3f27b81e481dc3124d55ffed523a839ee8446b64864","0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1","0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238","0x7f9d837cb17b0d36320ffe93ba52345f1b728571a568265caac97559dbc952a","0x2b94cf5e8746b3f5c9631f4c5df32907a699c58c94b2ad4d7b5cec1639183f55","0x2dee93c5a666459646ea7d22cca9e1bcfed71e6951b953611d11dda32ea09d78","0x78295e5a22b84e982cf601eb639597b8b0515a88cb5ac7fa8a4aabe3c87349d","0x2fa5e5f18f6027a6501bec864564472a616b2e274a41211a444cbe3a99f3cc61","0xe884376d0d8fd21ecb780389e941f66e45e7acce3e228ab3e2156a614fcd747","0x1b7201da72494f1e28717ad1a52eb469f95892f957713533de6175e5da190af2","0x1f8d8822725e36385200c0b201249819a6e6e1e4650808b5bebc6bface7d7636","0x2c5d82f66c914bafb9701589ba8cfcfb6162b0a12acf88a8d0879a0471b5f85a","0x14c54148a0940bb820957f5adf3fa1134ef5c4aaa113f4646458f270e0bfbfd0","0x190d33b12f986f961e10c0ee44d8b9af11be25588cad89d416118e4bf4ebe80c","0x22f98aa9ce704152ac17354914ad73ed1167ae6596af510aa5b3649325e06c92","0x2a7c7c9b6ce5880b9f6f228d72bf6a575a526f29c66ecceef8b753d38bba7323","0x2e8186e558698ec1c67af9c14d463ffc470043c9c2988b954d75dd643f36b992","0xf57c5571e9a4eab49e2c8cf050dae948aef6ead647392273546249d1c1ff10f","0x1830ee67b5fb554ad5f63d4388800e1cfe78e310697d46e43c9ce36134f72cca","0x2134e76ac5d21aab186c2be1dd8f84ee880a1e46eaf712f9d371b6df22191f3e","0x19df90ec844ebc4ffeebd866f33859b0c051d8c958ee3aa88f8f8df3db91a5b1","0x18cca2a66b5c0787981e69aefd84852d74af0e93ef4912b4648c05f722efe52b","0x2388909415230d1b4d1304d2d54f473a628338f2efad83fadf05644549d2538d","0x27171fb4a97b6cc0e9e8f543b5294de866a2af2c9c8d0b1d96e673e4529ed540","0x2ff6650540f629fd5711a0bc74fc0d28dcb230b9392583e5f8d59696dde6ae21"]],"leaf":["0x29176100eaa962bdc1fe6c654d6a3c130e96a4d1168b33848b897dc502820133"]}
`
	response, err := http.Post("http://"+ProverAddress+"/inclusion", "application/json", strings.NewReader(testInput))
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
	{"root":["0x0"],"inPathIndices":[0],"inPathElements":[["0x1","0x2098f5fb9e239eab3ceac3f27b81e481dc3124d55ffed523a839ee8446b64864","0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1","0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238","0x7f9d837cb17b0d36320ffe93ba52345f1b728571a568265caac97559dbc952a","0x2b94cf5e8746b3f5c9631f4c5df32907a699c58c94b2ad4d7b5cec1639183f55","0x2dee93c5a666459646ea7d22cca9e1bcfed71e6951b953611d11dda32ea09d78","0x78295e5a22b84e982cf601eb639597b8b0515a88cb5ac7fa8a4aabe3c87349d","0x2fa5e5f18f6027a6501bec864564472a616b2e274a41211a444cbe3a99f3cc61","0xe884376d0d8fd21ecb780389e941f66e45e7acce3e228ab3e2156a614fcd747","0x1b7201da72494f1e28717ad1a52eb469f95892f957713533de6175e5da190af2","0x1f8d8822725e36385200c0b201249819a6e6e1e4650808b5bebc6bface7d7636","0x2c5d82f66c914bafb9701589ba8cfcfb6162b0a12acf88a8d0879a0471b5f85a","0x14c54148a0940bb820957f5adf3fa1134ef5c4aaa113f4646458f270e0bfbfd0","0x190d33b12f986f961e10c0ee44d8b9af11be25588cad89d416118e4bf4ebe80c","0x22f98aa9ce704152ac17354914ad73ed1167ae6596af510aa5b3649325e06c92","0x2a7c7c9b6ce5880b9f6f228d72bf6a575a526f29c66ecceef8b753d38bba7323","0x2e8186e558698ec1c67af9c14d463ffc470043c9c2988b954d75dd643f36b992","0xf57c5571e9a4eab49e2c8cf050dae948aef6ead647392273546249d1c1ff10f","0x1830ee67b5fb554ad5f63d4388800e1cfe78e310697d46e43c9ce36134f72cca","0x2134e76ac5d21aab186c2be1dd8f84ee880a1e46eaf712f9d371b6df22191f3e","0x19df90ec844ebc4ffeebd866f33859b0c051d8c958ee3aa88f8f8df3db91a5b1","0x18cca2a66b5c0787981e69aefd84852d74af0e93ef4912b4648c05f722efe52b","0x2388909415230d1b4d1304d2d54f473a628338f2efad83fadf05644549d2538d","0x27171fb4a97b6cc0e9e8f543b5294de866a2af2c9c8d0b1d96e673e4529ed540","0x2ff6650540f629fd5711a0bc74fc0d28dcb230b9392583e5f8d59696dde6ae21"]],"leaf":["0x29176100eaa962bdc1fe6c654d6a3c130e96a4d1168b33848b897dc502820133"]}
`

	response, err := http.Post("http://"+ProverAddress+"/inclusion", "application/json", strings.NewReader(testInput))
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
	{"root":["0xeb3a12b56841d019b26e4452bf7db8cf474a2d63e21781c5d2f143db039924a"],
	"inPathIndices":[0],
	"inPathElements":[
		["0x0",
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
		"0x19df90ec844ebc4ffeebd866f33859b0c051d8c958ee3aa88f8f8df3db91a5b1"]
	],
	"leaf":["0x29176100eaa962bdc1fe6c654d6a3c130e96a4d1168b33848b897dc502820133"]
	}`

	proofData, err := prover.ParseInput(testInput)
	if err != nil {
		t.Errorf("error parsing input: %v", err)
	}

	tree := merkletree.BuildTestTree(22, 1, false)

	if len(tree.Root) != len(proofData.Root) || len(tree.Leaf) != len(proofData.Leaf) || len(tree.InPathElements) != len(proofData.InPathElements) || len(tree.InPathIndices) != len(proofData.InPathIndices) {
		t.Errorf("Invalid shape: expected %d, got %d", len(tree.Root), len(proofData.Root))
	}

	for i, bi := range tree.Root {
		if bi.String() != proofData.Root[i].String() {
			t.Errorf("Invalid root: expected %s, got %s", bi.String(), proofData.Root[i].String())
		}
	}

	for i, bi := range tree.Leaf {
		if bi.String() != proofData.Leaf[i].String() {
			t.Errorf("Invalid leaf: expected %s, got %s", bi.String(), proofData.Leaf[i].String())
		}
	}

	for i, bi := range tree.InPathIndices {
		if bi != proofData.InPathIndices[i] {
			t.Errorf("Invalid inPathIndices: expected %d, got %d", bi, proofData.InPathIndices[i])
		}
	}

	for i, bi := range tree.InPathElements {
		for j, bj := range bi {
			if bj.String() != proofData.InPathElements[i][j].String() {
				t.Errorf("Invalid inPathElements: expected %s, got %s", bj.String(), proofData.InPathElements[i][j].String())
			}
		}
	}
}
