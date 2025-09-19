package v2

import (
	"math/big"
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/test"
)

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
