package prover

import (
	"encoding/json"
	"math/big"
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/test"
)

func IgnoredTestAddressAppendHardcoded4_1(t *testing.T) {
	assert := test.NewAssert(t)

	circuit := InitBatchAddressTreeAppendCircuit(4, 1)

	params := get_test_data_1_insert()
	witness, err := params.CreateWitness()
	if err != nil {
		t.Fatal(err)
	}

	err = test.IsSolved(&circuit, witness, ecc.BN254.ScalarField())
	assert.NoError(err)
}

func IgnoredTestAddressAppendHardcoded4_2(t *testing.T) {
	assert := test.NewAssert(t)

	circuit := InitBatchAddressTreeAppendCircuit(4, 2)
	params := get_test_data_2_insert()
	witness, err := params.CreateWitness()
	if err != nil {
		t.Fatal(err)
	}

	err = test.IsSolved(&circuit, witness, ecc.BN254.ScalarField())
	assert.NoError(err)
}

func TestBatchAddressAppendCircuit(t *testing.T) {
	assert := test.NewAssert(t)

	t.Run("Basic operations", func(t *testing.T) {
		testCases := []struct {
			name       string
			treeHeight uint32
			batchSize  uint32
			startIndex uint32
			shouldPass bool
		}{
			{"Single insert height 4", 4, 1, 2, true},
			{"Batch insert height 4", 4, 2, 2, true},
			{"Single insert height 8", 8, 1, 2, true},
			{"Large batch height 8", 8, 4, 2, true},
		}

		for _, tc := range testCases {
			t.Run(tc.name, func(t *testing.T) {
				circuit := InitBatchAddressTreeAppendCircuit(tc.treeHeight, tc.batchSize)

				params, err := BuildTestAddressTree(tc.treeHeight, tc.batchSize, tc.startIndex)
				if err != nil {
					t.Fatalf("Failed to build test tree: %v", err)
				}

				witness, err := params.CreateWitness()
				if err != nil {
					t.Fatalf("Failed to create witness: %v", err)
				}

				err = test.IsSolved(&circuit, witness, ecc.BN254.ScalarField())
				if tc.shouldPass {
					assert.NoError(err)
				} else {
					assert.Error(err)
				}
			})
		}
	})

	t.Run("Invalid cases", func(t *testing.T) {
		testCases := []struct {
			name         string
			treeHeight   uint32
			batchSize    uint32
			startIndex   uint32
			modifyParams func(*BatchAddressAppendParameters)
		}{
			{
				name:       "Invalid OldRoot",
				treeHeight: 4,
				batchSize:  1,
				startIndex: 0,
				modifyParams: func(p *BatchAddressAppendParameters) {
					p.OldRoot.Add(p.OldRoot, big.NewInt(1))
				},
			},
			{
				name:       "Invalid NewRoot",
				treeHeight: 4,
				batchSize:  1,
				startIndex: 0,
				modifyParams: func(p *BatchAddressAppendParameters) {
					p.NewRoot.Add(p.NewRoot, big.NewInt(1))
				},
			},
			{
				name:       "Invalid HashchainHash",
				treeHeight: 4,
				batchSize:  1,
				startIndex: 0,
				modifyParams: func(p *BatchAddressAppendParameters) {
					p.HashchainHash.Add(p.HashchainHash, big.NewInt(1))
				},
			},
			{
				name:       "Invalid LowElementValue",
				treeHeight: 4,
				batchSize:  1,
				startIndex: 0,
				modifyParams: func(p *BatchAddressAppendParameters) {
					p.LowElementValues[0].Add(&p.LowElementValues[0], big.NewInt(1))
				},
			},
		}

		for _, tc := range testCases {
			t.Run(tc.name, func(t *testing.T) {
				circuit := InitBatchAddressTreeAppendCircuit(tc.treeHeight, tc.batchSize)

				params, err := BuildTestAddressTree(tc.treeHeight, tc.batchSize, tc.startIndex)
				if err != nil {
					t.Fatalf("Failed to build test tree: %v", err)
				}

				tc.modifyParams(params)

				witness, err := params.CreateWitness()
				if err != nil {
					return
				}

				err = test.IsSolved(&circuit, witness, ecc.BN254.ScalarField())
				assert.Error(err)
			})
		}
	})

}

func get_test_data_1_insert() BatchAddressAppendParameters {
	jsonData := `{
		"PublicInputHash": "636798750499118125492639857559342304110421620753013517995729792767289738238",
		"OldRoot": "4088321652280896297689618452575487697996974475167705659475507189280537614421",
		"NewRoot": "9360296153682585144477573401014216227476090504161963538926993860213587505773",
		"HashchainHash": "30",
		"StartIndex": 2,
		"LowElementValues": ["0"],
		"LowElementIndices": ["0"],
		"LowElementNextIndices": ["1"],
		"LowElementNextValues": ["452312848583266388373324160190187140051835877600158453279131187530910662655"],
		"NewElementValues": ["30"],
		"LowElementProofs": [
			["13859306649965657812382249699983066845935552967038026417581136538215435035637", 
			"14744269619966411208579211824598458697587494354926760081771325075741142829156", 
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

	params, err := ParseBatchAddressAppendInput(jsonData)
	if err != nil {
		panic(err)
	}
	return params
}

func get_test_data_2_insert() BatchAddressAppendParameters {
	jsonData := `{
	"publicInputHash": "1403572307386503039346238276855266353384715296309161920440230822038749882432",
	"oldRoot": "4088321652280896297689618452575487697996974475167705659475507189280537614421",
	"newRoot": "19171761698158463304851719404211649373247489649232357264671218893697063910840",
	"hashchainHash": "13832493800585898166114088680240082093515072733861191400485854249726904407056",
	"startIndex": 2,
	"lowElementValues": ["0", "0"],
	"lowElementIndices": ["0", "0"],
	"lowElementNextIndices": ["1", "2"],
	"lowElementNextValues": [
	  "452312848583266388373324160190187140051835877600158453279131187530910662655",
	  "31"
	],
	"newElementValues": ["31", "30"],
	"lowElementProofs": [
	  [
		"13859306649965657812382249699983066845935552967038026417581136538215435035637",
		"14744269619966411208579211824598458697587494354926760081771325075741142829156",
		"7423237065226347324353380772367382631490014989348495481811164164159255474657",
		"11286972368698509976183087595462810875513684078608517520839298933882497716792"
	  ],
	  [
		"13859306649965657812382249699983066845935552967038026417581136538215435035637",
		"3796870957066466565085934353165460010672460214992313636808730505094443732049",
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
	params := BatchAddressAppendParameters{
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
