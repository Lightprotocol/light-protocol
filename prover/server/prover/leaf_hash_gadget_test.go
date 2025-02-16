package prover

import (
	"math/big"
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/test"
)

type LeafHashGadgetCircuit struct {
	LeafLowerRangeValue  frontend.Variable `gnark:"private"`
	LeafHigherRangeValue frontend.Variable `gnark:"private"`
	Value                frontend.Variable `gnark:"private"`
	NextIndex            frontend.Variable `gnark:"private"`
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
	nextIndexStr := 1
	valueStr := "277f5629fdf020bb57ecbf7024ba4a9c26b9de1cda2ca25bfd3dc94275996e"
	leafLowerRangeValue := big.NewInt(0)
	leafHigherRangeValue := new(big.Int)
	leafHigherRangeValue.SetString(leafHigherRangeValueStr, 16)
	nextIndex := big.NewInt(int64(nextIndexStr))
	value := big.NewInt(0)
	value.SetString(valueStr, 16)
	ExpectedHashStr := "18107977475760319057966144103673937810858686565134338371146286848755066863726"
	ExpectedHash := new(big.Int)
	ExpectedHash.SetString(ExpectedHashStr, 10)
	invalid_value := new(big.Int)
	invalid_value.SetString("18107977475760319057966144103673937810858686565134338371146286848755066863725", 10)
	testCases := []struct {
		LeafLowerRangeValue  *big.Int
		LeafHigherRangeValue *big.Int
		Value                *big.Int
		NextIndex            *big.Int
		ExpectedHash         *big.Int
		expected             bool
	}{
		{
			leafLowerRangeValue,
			leafHigherRangeValue,
			value,
			nextIndex,
			ExpectedHash,
			true,
		},
		{
			leafLowerRangeValue,
			leafHigherRangeValue,
			value,
			nextIndex,
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
				NextIndex:            tc.NextIndex,
				ExpectedHash:         tc.ExpectedHash,
			}, test.WithBackends(backend.GROTH16), test.WithCurves(ecc.BN254), test.NoSerialization())
		} else {
			assert := test.NewAssert(t)
			assert.ProverFailed(&circuit, &LeafHashGadgetCircuit{
				LeafLowerRangeValue:  tc.LeafLowerRangeValue,
				LeafHigherRangeValue: tc.LeafHigherRangeValue,
				Value:                tc.Value,
				NextIndex:            tc.NextIndex,
				ExpectedHash:         tc.ExpectedHash,
			}, test.WithBackends(backend.GROTH16), test.WithCurves(ecc.BN254), test.NoSerialization())
		}
	}
}
