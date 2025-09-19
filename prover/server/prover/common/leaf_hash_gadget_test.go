package common

import (
	"math/big"
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/test"
	"github.com/iden3/go-iden3-crypto/poseidon"
)

type LeafHashGadgetCircuit struct {
	LeafLowerRangeValue  frontend.Variable `gnark:"private"`
	LeafHigherRangeValue frontend.Variable `gnark:"private"`
	Value                frontend.Variable `gnark:"private"`
	ExpectedHash         frontend.Variable `gnark:"public"`
}

func (circuit *LeafHashGadgetCircuit) Define(api frontend.API) error {
	input := LeafHashGadget{
		LeafLowerRangeValue:  circuit.LeafLowerRangeValue,
		LeafHigherRangeValue: circuit.LeafHigherRangeValue,
		Value:                circuit.Value,
	}
	output := LeafHashGadget(input).DefineGadget(api)
	api.AssertIsEqual(circuit.ExpectedHash, output)
	return nil
}

func TestLeafGadget(t *testing.T) {
	// Test cases
	leafHigherRangeValueStr := "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
	valueStr := "277f5629fdf020bb57ecbf7024ba4a9c26b9de1cda2ca25bfd3dc94275996e"
	leafLowerRangeValue := big.NewInt(0)
	leafHigherRangeValue := new(big.Int)
	leafHigherRangeValue.SetString(leafHigherRangeValueStr, 16)
	value := big.NewInt(0)
	value.SetString(valueStr, 16)
	ExpectedHashStr, _ := poseidon.Hash([]*big.Int{leafLowerRangeValue, leafHigherRangeValue})
	ExpectedHash := new(big.Int)
	ExpectedHash.SetString(ExpectedHashStr.String(), 10)
	invalid_value := new(big.Int)
	invalid_value.SetString("18107977475760319057966144103673937810858686565134338371146286848755066863725", 10)
	testCases := []struct {
		LeafLowerRangeValue  *big.Int
		LeafHigherRangeValue *big.Int
		Value                *big.Int
		ExpectedHash         *big.Int
		expected             bool
	}{
		{
			leafLowerRangeValue,
			leafHigherRangeValue,
			value,
			ExpectedHash,
			true,
		},
		{
			leafLowerRangeValue,
			leafHigherRangeValue,
			value,
			invalid_value,
			false,
		},
	}

	for _, tc := range testCases {
		var circuit LeafHashGadgetCircuit
		if tc.expected {
			assert := test.NewAssert(t)
			assert.ProverSucceeded(&circuit, &LeafHashGadgetCircuit{
				LeafLowerRangeValue:  tc.LeafLowerRangeValue,
				LeafHigherRangeValue: tc.LeafHigherRangeValue,
				Value:                tc.Value,
				ExpectedHash:         tc.ExpectedHash,
			}, test.WithBackends(backend.GROTH16), test.WithCurves(ecc.BN254), test.NoSerializationChecks())
		} else {
			assert := test.NewAssert(t)
			assert.ProverFailed(&circuit, &LeafHashGadgetCircuit{
				LeafLowerRangeValue:  tc.LeafLowerRangeValue,
				LeafHigherRangeValue: tc.LeafHigherRangeValue,
				Value:                tc.Value,
				ExpectedHash:         tc.ExpectedHash,
			}, test.WithBackends(backend.GROTH16), test.WithCurves(ecc.BN254), test.NoSerializationChecks())
		}
	}
}
