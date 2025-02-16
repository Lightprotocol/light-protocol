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
			startIndex uint64
			shouldPass bool
		}{
			{"Single insert height 4", 4, 1, 1, true},
			{"Batch insert height 4", 4, 2, 1, true},
			{"Single insert height 8", 8, 1, 1, true},
			{"Large batch height 8", 8, 4, 1, true},
		}

		for _, tc := range testCases {
			t.Run(tc.name, func(t *testing.T) {
				circuit := InitBatchAddressTreeAppendCircuit(tc.treeHeight, tc.batchSize)

				params, err := BuildTestAddressTree(tc.treeHeight, tc.batchSize, nil, tc.startIndex)
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
			startIndex   uint64
			modifyParams func(*BatchAddressAppendParameters)
			wantPanic    bool
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
			{
				name:       "StartIndex too large",
				treeHeight: 4,
				batchSize:  1,
				startIndex: 0,
				modifyParams: func(p *BatchAddressAppendParameters) {
					p.StartIndex = ^uint64(0)
				},
			},
			{
				name:       "Mismatched array length",
				treeHeight: 4,
				batchSize:  2,
				startIndex: 0,
				modifyParams: func(p *BatchAddressAppendParameters) {
					p.LowElementValues = p.LowElementValues[:len(p.LowElementValues)-1]
				},
				wantPanic: true,
			},
			{
				name:       "Invalid proof length",
				treeHeight: 4,
				batchSize:  2,
				startIndex: 0,
				modifyParams: func(p *BatchAddressAppendParameters) {
					p.LowElementProofs[0] = p.LowElementProofs[0][:len(p.LowElementProofs[0])-1]
				},
				wantPanic: true,
			},
			{
				name:       "Empty arrays",
				treeHeight: 4,
				batchSize:  2,
				startIndex: 0,
				modifyParams: func(p *BatchAddressAppendParameters) {
					p.LowElementValues = make([]big.Int, p.BatchSize)
					p.NewElementValues = make([]big.Int, p.BatchSize)
				},
			},
			{
				name:       "Max values",
				treeHeight: 4,
				batchSize:  1,
				startIndex: 0,
				modifyParams: func(p *BatchAddressAppendParameters) {
					maxBigInt := new(big.Int).Sub(new(big.Int).Exp(big.NewInt(2), big.NewInt(256), nil), big.NewInt(1))
					p.NewElementValues[0] = *maxBigInt
				},
			},
			{
				name:       "Inconsistent start index with proofs",
				treeHeight: 4,
				batchSize:  1,
				startIndex: 0,
				modifyParams: func(p *BatchAddressAppendParameters) {
					p.StartIndex = 5
				},
			},
			{
				name:       "Low element below expected range",
				treeHeight: 4,
				batchSize:  1,
				startIndex: 1,
				modifyParams: func(p *BatchAddressAppendParameters) {
					p.LowElementValues[0].Sub(&p.LowElementValues[0], big.NewInt(1))
				},
			},
			{
				name:       "Low element above expected range",
				treeHeight: 4,
				batchSize:  1,
				startIndex: 1,
				modifyParams: func(p *BatchAddressAppendParameters) {
					// Set low element value above valid range
					maxVal := new(big.Int).Exp(big.NewInt(2), big.NewInt(256), nil)
					p.LowElementValues[0].Add(&p.LowElementValues[0], maxVal)
				},
			},
			{
				name:       "Invalid low element next values",
				treeHeight: 4,
				batchSize:  1,
				startIndex: 1,
				modifyParams: func(p *BatchAddressAppendParameters) {
					p.LowElementNextValues[0].Add(&p.LowElementNextValues[0], big.NewInt(1))
				},
			},
			{
				name:       "Invalid low element indices",
				treeHeight: 4,
				batchSize:  1,
				startIndex: 1,
				modifyParams: func(p *BatchAddressAppendParameters) {
					p.LowElementIndices[0].Add(&p.LowElementIndices[0], big.NewInt(3))
				},
			},
			{
				name:       "Invalid low element proofs",
				treeHeight: 4,
				batchSize:  1,
				startIndex: 1,
				modifyParams: func(p *BatchAddressAppendParameters) {
					p.LowElementProofs[0][0].Add(&p.LowElementProofs[0][0], big.NewInt(1))
				},
			},
			{
				name:       "Invalid new element proofs",
				treeHeight: 4,
				batchSize:  1,
				startIndex: 1,
				modifyParams: func(p *BatchAddressAppendParameters) {
					p.NewElementProofs[0][0].Add(&p.NewElementProofs[0][0], big.NewInt(1))
				},
			},
			{
				name:       "Invalid new element values",
				treeHeight: 4,
				batchSize:  1,
				startIndex: 1,
				modifyParams: func(p *BatchAddressAppendParameters) {
					p.NewElementValues[0].Add(&p.NewElementValues[0], big.NewInt(1))
				},
			},
		}

		for _, tc := range testCases {
			t.Run(tc.name, func(t *testing.T) {
				circuit := InitBatchAddressTreeAppendCircuit(tc.treeHeight, tc.batchSize)

				params, err := BuildTestAddressTree(tc.treeHeight, tc.batchSize, nil, tc.startIndex)
				if err != nil {
					t.Fatalf("Failed to build test tree: %v", err)
				}

				tc.modifyParams(params)

				if tc.wantPanic {
					assert.Panics(func() {
						witness, _ := params.CreateWitness()
						test.IsSolved(&circuit, witness, ecc.BN254.ScalarField())
					})
					return
				}

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
	jsonData := `
{
  "BatchSize": 1,
  "HashchainHash": "0x1e",
  "LowElementIndices": [
    "0x0"
  ],
  "LowElementNextIndices": [
    "0x1"
  ],
  "LowElementNextValues": [
    "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
  ],
  "LowElementProofs": [
    [
      "0x1ea416eeb40218b540c1cfb8dbe91f6d54e8a29edc30a39e326b4057a7d963f5",
      "0x2098f5fb9e239eab3ceac3f27b81e481dc3124d55ffed523a839ee8446b64864",
      "0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1",
      "0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238"
    ]
  ],
  "LowElementValues": [
    "0x0"
  ],
  "NewElementProofs": [
    [
      "0x0",
      "0x12e5c92ca57654ded1d93934a93505ce14ae3ed617c7f934673c1d3830975760",
      "0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1",
      "0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238"
    ]
  ],
  "NewElementValues": [
    "0x1e"
  ],
  "NewRoot": "0x14b1bd68a7aaf8db72124dbdefd41c495565c5050b31c465f3407a6b1e3ef26d",
  "OldRoot": "0x909e8762fb09c626001b19f6441a2cd2da21b1622c6970ec9c4863ec9c09855",
  "PublicInputHash": "0x1686a526bc791be496f67a405f2c3cfc0f86b6c6dcec6e05dff5a6285c043fe",
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
  "HashchainHash": "0x1e94e9fed8440d50ff872bedcc6a6c460f9c6688ac167f68e288057e63109410",
  "LowElementIndices": [
    "0x0",
    "0x0"
  ],
  "LowElementNextIndices": [
    "0x1",
    "0x2"
  ],
  "LowElementNextValues": [
    "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
    "0x1f"
  ],
  "LowElementProofs": [
    [
      "0x1ea416eeb40218b540c1cfb8dbe91f6d54e8a29edc30a39e326b4057a7d963f5",
      "0x2098f5fb9e239eab3ceac3f27b81e481dc3124d55ffed523a839ee8446b64864",
      "0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1",
      "0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238"
    ],
    [
      "0x1ea416eeb40218b540c1cfb8dbe91f6d54e8a29edc30a39e326b4057a7d963f5",
      "0x864f3eb12bb83a5cdc9ff6fdc8b985aa4b87292c5eef49201065277170e8c51",
      "0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1",
      "0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238"
    ]
  ],
  "LowElementValues": [
    "0x0",
    "0x0"
  ],
  "NewElementProofs": [
    [
      "0x0",
      "0x2cfd59ee6c304f7f1e82d9e7e857a380e991fb02728f09324baffef2807e74fa",
      "0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1",
      "0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238"
    ],
    [
      "0x29794d28dddbdb020ec3974ecc41bcf64fb695eb222bde71f2a130e92852c0eb",
      "0x15920e98b921491171b9b2b0a8ac1545e10b58e9c058822b6de9f4179bbd2e7c",
      "0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1",
      "0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238"
    ]
  ],
  "NewElementValues": [
    "0x1f",
    "0x1e"
  ],
  "NewRoot": "0x2a62d5241a6d3659df612b996ad729abe32f425bfec249f060983013ba2cfdb8",
  "OldRoot": "0x909e8762fb09c626001b19f6441a2cd2da21b1622c6970ec9c4863ec9c09855",
  "PublicInputHash": "0x31a64ce5adc664d1092fd7353a76b4fe0a3e63ad0cf313d66a6bc89e5e4a840",
  "StartIndex": 2,
  "TreeHeight": 4
}`

	params, err := ParseBatchAddressAppendInput(jsonData)
	if err != nil {
		panic(err)
	}
	return params
}
