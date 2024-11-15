package prover

import (
	"math/big"
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/test"
)

func TestAddressAppendHardcoded4_1(t *testing.T) {
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

func TestAddressAppendHardcoded4_2(t *testing.T) {
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
  "BatchSize": 1,
  "HashchainHash": "30",
  "LowElementValues": [
    "0"
  ],
  "LowElementIndices": [
    "0"
  ],
  "LowElementNextIndices": [
    "1"
  ],
  "LowElementNextValues": [
    "452312848583266388373324160190187140051835877600158453279131187530910662655"
  ],
  "LowElementProofs": [
    [
      "13859306649965657812382249699983066845935552967038026417581136538215435035637",
      "14744269619966411208579211824598458697587494354926760081771325075741142829156",
      "7423237065226347324353380772367382631490014989348495481811164164159255474657",
      "11286972368698509976183087595462810875513684078608517520839298933882497716792"
    ]
  ],
  "NewElementValues": [
    "30"
  ],
  "NewElementProofs": [
    [
      "0",
      "8547627707011610151354520719421304546275272960684542609603067758776562440032",
      "7423237065226347324353380772367382631490014989348495481811164164159255474657",
      "11286972368698509976183087595462810875513684078608517520839298933882497716792"
    ]
  ],
  "NewRoot": "9360296153682585144477573401014216227476090504161963538926993860213587505773",
  "OldRoot": "4088321652280896297689618452575487697996974475167705659475507189280537614421",
  "PublicInputHash": "636798750499118125492639857559342304110421620753013517995729792767289738238",
  "StartIndex": 2,
  "TreeHeight": 4
}
`

	params, err := ParseBatchAddressAppendInput(jsonData)
	if err != nil {
		panic(err)
	}
	return params
}

func get_test_data_2_insert() BatchAddressAppendParameters {

	jsonData := `{
  "BatchSize": 2,
  "HashchainHash": "9141339901581071536976622883688234652052589882443274304799219173891220176622",
  "LowElementValues": [
    "0",
    "30"
  ],
  "LowElementIndices": [
    "0",
    "2"
  ],
  "LowElementNextIndices": [
    "1",
    "1"
  ],
  "LowElementNextValues": [
    "452312848583266388373324160190187140051835877600158453279131187530910662655",
    "452312848583266388373324160190187140051835877600158453279131187530910662655"
  ],
  "LowElementProofs": [
    [
      "13859306649965657812382249699983066845935552967038026417581136538215435035637",
      "14744269619966411208579211824598458697587494354926760081771325075741142829156",
      "7423237065226347324353380772367382631490014989348495481811164164159255474657",
      "11286972368698509976183087595462810875513684078608517520839298933882497716792"
    ],
    [
      "0",
      "8547627707011610151354520719421304546275272960684542609603067758776562440032",
      "7423237065226347324353380772367382631490014989348495481811164164159255474657",
      "11286972368698509976183087595462810875513684078608517520839298933882497716792"
    ]
  ],
  "NewElementValues": [
    "30",
    "31"
  ],
  "NewElementProofs": [
    [
      "0",
      "8547627707011610151354520719421304546275272960684542609603067758776562440032",
      "7423237065226347324353380772367382631490014989348495481811164164159255474657",
      "11286972368698509976183087595462810875513684078608517520839298933882497716792"
    ],
    [
      "10485818150837015558398530757910609463875859082430070889536105699344336383718",
      "8547627707011610151354520719421304546275272960684542609603067758776562440032",
      "7423237065226347324353380772367382631490014989348495481811164164159255474657",
      "11286972368698509976183087595462810875513684078608517520839298933882497716792"
    ]
  ],
  "NewRoot": "14681622229506223316550031648881360593743863923218490088635688909074669941697",
  "OldRoot": "4088321652280896297689618452575487697996974475167705659475507189280537614421",
  "PublicInputHash": "9110422259749297267016076474620342264095561844435508436615697772347396704389",
  "StartIndex": 2,
  "TreeHeight": 4
}`

	params, err := ParseBatchAddressAppendInput(jsonData)
	if err != nil {
		panic(err)
	}
	return params
}
