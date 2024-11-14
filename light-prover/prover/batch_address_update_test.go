package prover

import (
	"encoding/json"
	"fmt"
	"math/big"
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/test"
)

func TestAddressAppendHardcoded4_1(t *testing.T) {
	assert := test.NewAssert(t)

	params := get_test_data_1_insert()
	circuit := createAddressCircuit(&params)
	witness := createAddressWitness(&params)

	err := test.IsSolved(circuit, witness, ecc.BN254.ScalarField())
	assert.NoError(err)
}

func createAddressCircuit(params *BatchAddressTreeAppendParameters) *BatchAddressTreeAppendCircuit {
	if params == nil {
		panic("params cannot be nil")
	}
	lowElementProofs := make([][]frontend.Variable, params.BatchSize)
	newElementProofs := make([][]frontend.Variable, params.BatchSize)

	for i := 0; i < int(params.BatchSize); i++ {
		lowElementProofs[i] = make([]frontend.Variable, params.TreeHeight)
		newElementProofs[i] = make([]frontend.Variable, params.TreeHeight)
	}

	circuit := &BatchAddressTreeAppendCircuit{
		PublicInputHash: frontend.Variable(0),
		OldRoot:         frontend.Variable(0),
		NewRoot:         frontend.Variable(0),
		HashchainHash:   frontend.Variable(0),
		StartIndex:      frontend.Variable(0),

		LowElementValues:      make([]frontend.Variable, params.BatchSize),
		LowElementNextValues:  make([]frontend.Variable, params.BatchSize),
		LowElementNextIndices: make([]frontend.Variable, params.BatchSize),
		LowElementIndices:     make([]frontend.Variable, params.BatchSize),
		LowElementProofs:      lowElementProofs,

		NewElementValues: make([]frontend.Variable, params.BatchSize),
		NewElementProofs: newElementProofs,

		BatchSize:  params.BatchSize,
		TreeHeight: params.TreeHeight,
	}

	return circuit
}

func createAddressWitness(params *BatchAddressTreeAppendParameters) *BatchAddressTreeAppendCircuit {
	witness := createAddressCircuit(params)

	witness.PublicInputHash = frontend.Variable(params.PublicInputHash)
	witness.OldRoot = params.OldRoot
	witness.NewRoot = params.NewRoot
	witness.HashchainHash = frontend.Variable(params.HashchainHash)
	witness.StartIndex = frontend.Variable(params.StartIndex)
	fmt.Println("createAddressWitness params.BatchSize ", params.BatchSize)
	for i := uint32(0); i < params.BatchSize; i++ {
		witness.LowElementValues[i] = frontend.Variable(params.LowElementValues[i])
		witness.LowElementIndices[i] = frontend.Variable(params.LowElementIndices[i])
		witness.LowElementNextIndices[i] = frontend.Variable(params.LowElementNextIndices[i])
		witness.LowElementNextValues[i] = frontend.Variable(params.LowElementNextValues[i])

		witness.NewElementValues[i] = frontend.Variable(params.NewElementValues[i])
		witness.LowElementProofs[i] = make([]frontend.Variable, len(params.LowElementProofs[i]))
		witness.NewElementProofs[i] = make([]frontend.Variable, len(params.NewElementProofs[i]))

		for j := 0; j < len(params.LowElementProofs[i]); j++ {
			witness.LowElementProofs[i][j] = frontend.Variable(params.LowElementProofs[i][j])
		}
		for j := 0; j < len(params.NewElementProofs[i]); j++ {
			witness.NewElementProofs[i][j] = frontend.Variable(params.NewElementProofs[i][j])
		}

	}

	return witness
}

type JsonBatchAddressTreeAppendParameters struct {
	PublicInputHash       string     `json:"PublicInputHash"`
	OldRoot               string     `json:"OldRoot"`
	NewRoot               string     `json:"NewRoot"`
	HashchainHash         string     `json:"HashchainHash"`
	StartIndex            uint32     `json:"StartIndex"`
	LowElementValues      []string   `json:"LowElementValues"`
	LowElementIndices     []string   `json:"LowElementIndices"`
	LowElementNextIndices []string   `json:"LowElementNextIndices"`
	LowElementNextValues  []string   `json:"LowElementNextValues"`
	NewElementValues      []string   `json:"NewElementValues"`
	LowElementProofs      [][]string `json:"LowElementProofs"`
	NewElementProofs      [][]string `json:"NewElementProofs"`
	TreeHeight            string     `json:"TreeHeight"`
	BatchSize             string     `json:"BatchSize"`
}

func get_test_data_1_insert() BatchAddressTreeAppendParameters {
	jsonData := `{
		"PublicInputHash": "4088321652280896297689618452575487697996974475167705659475507189280537614421",
		"OldRoot": "4088321652280896297689618452575487697996974475167705659475507189280537614421",
		"NewRoot": "9360296153682585144477573401014216227476090504161963538926993860213587505773",
		"HashchainHash": "303229927723846428264111808645197890460298805508752674790354860916280465234",
		"StartIndex": 2,
		"LowElementValues": ["0"],
		"LowElementIndices": ["0"],
		"LowElementNextIndices": ["1"],
		"LowElementNextValues": ["452312848583266388373324160190187140051835877600158453279131187530910662655"],
		"NewElementValues": ["0"],
		"LowElementProofs": [
			["13859306649965657812382249699983066845935552967038026417581136538215435035637", 
			"15723694721673876141054887912220565198608814743306664888649577252769766605905", 
			"7423237065226347324353380772367382631490014989348495481811164164159255474657", 
			"11286972368698509976183087595462810875513684078608517520839298933882497716792"]
		],
		"NewElementProofs": [
			[
				"0",
				"8547627707011610151354520719421304546275272960684542609603067758776562440032",
				"7423237065226347324353380772367382631490014989348495481811164164159255474657",
				"11286972368698509976183087595462810875513684078608517520839298933882497716792"
			]
		],
		"TreeHeight": 4,
		"BatchSize": 1
	}`
	batchSize := 1
	params := BatchAddressTreeAppendParameters{
		PublicInputHash: big.NewInt(0),
		OldRoot:         big.NewInt(0),
		NewRoot:         big.NewInt(0),
		HashchainHash:   big.NewInt(0),
		StartIndex:      uint32(0),
		// Elements being modified or added
		LowElementValues:      make([]big.Int, batchSize),
		LowElementIndices:     make([]big.Int, batchSize),
		LowElementNextIndices: make([]big.Int, batchSize),
		LowElementNextValues:  make([]big.Int, batchSize),

		NewElementValues: make([]big.Int, batchSize),

		// Merkle proofs for verification
		LowElementProofs: make([][]big.Int, batchSize),
		NewElementProofs: make([][]big.Int, batchSize),

		// Tree configuration
		TreeHeight: 4,
		BatchSize:  1,
	}

	json.Unmarshal([]byte(jsonData), &params)
	return params
}

func get_test_data_2_insert() BatchAddressTreeAppendParameters {
	jsonData := `{
	"publicInputHash": "13832493800585898166114088680240082093515072733861191400485854249726904407056",
	"oldRoot": "4088321652280896297689618452575487697996974475167705659475507189280537614421",
	"newRoot": "19171761698158463304851719404211649373247489649232357264671218893697063910840",
	"hashchainHash": "13832493800585898166114088680240082093515072733861191400485854249726904407056",
	"startIndex": 0,
	"lowElementValues": ["0", "0"],
	"lowElementIndices": ["0", "0"],
	"lowElementNextIndices": ["1", "2"],
	"lowElementNextValues": [
	  "452312848583266388373324160190187140051835877600158453279131187530910662655",
	  "31"
	],
	"newElementValues": ["0", "18759147822983135752235477210489921219029732284985839751594385139872277119211"],
	"lowElementProofs": [
	  [
		"13859306649965657812382249699983066845935552967038026417581136538215435035637",
		"3796870957066466565085934353165460010672460214992313636808730505094443732049",
		"7423237065226347324353380772367382631490014989348495481811164164159255474657",
		"11286972368698509976183087595462810875513684078608517520839298933882497716792"
	  ],
	  [
		"13859306649965657812382249699983066845935552967038026417581136538215435035637",
		"9326699937298322758647210248953324788180879800245446112806190282402299353280",
		"7423237065226347324353380772367382631490014989348495481811164164159255474657",
		"11286972368698509976183087595462810875513684078608517520839298933882497716792"
	  ]
	],
	"newElementProofs": [
	  [
		"0",
		"20349398328338424766777768544456194850476422282930688296176215598270154437882",
		"7423237065226347324353380772367382631490014989348495481811164164159255474657",
		"11286972368698509976183087595462810875513684078608517520839298933882497716792"
	  ],
	  [
		"18759147822983135752235477210489921219029732284985839751594385139872277119211",
		"9756630233563453530006719845367697306884777538379147077278822652368465505916",
		"7423237065226347324353380772367382631490014989348495481811164164159255474657",
		"11286972368698509976183087595462810875513684078608517520839298933882497716792"
	  ]
	],
	"treeHeight": 4,
	"batchSize": 2
  }`
	batchSize := 1
	params := BatchAddressTreeAppendParameters{
		PublicInputHash: big.NewInt(0),
		OldRoot:         big.NewInt(0),
		NewRoot:         big.NewInt(0),
		HashchainHash:   big.NewInt(0),
		StartIndex:      uint32(0),

		LowElementValues:      make([]big.Int, batchSize),
		LowElementIndices:     make([]big.Int, batchSize),
		LowElementNextIndices: make([]big.Int, batchSize),
		LowElementNextValues:  make([]big.Int, batchSize),
		NewElementValues:      make([]big.Int, batchSize),

		LowElementProofs: make([][]big.Int, batchSize),
		NewElementProofs: make([][]big.Int, batchSize),

		TreeHeight: 4,
		BatchSize:  1,
	}

	json.Unmarshal([]byte(jsonData), &params)
	return params
}
