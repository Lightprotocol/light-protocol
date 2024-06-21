package prover

import (
	"math/big"
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/test"
)

type IsLessCircuit struct {
	A frontend.Variable `gnark:"public"`
	B frontend.Variable `gnark:"private"`
}

func (circuit *IsLessCircuit) Define(api frontend.API) error {
	isLess := AssertIsLess{
		A: circuit.A,
		B: circuit.B,
		N: 248,
	}
	AssertIsLess(isLess).DefineGadget(api)
	return nil
}

func TestAssertIsLess(t *testing.T) {
	fieldSizeStr := "21888242871839275222246405745257275088548364400416034343698204186575808495617"
	fieldSizeSub1Str := "21888242871839275222246405745257275088548364400416034343698204186575808495616"

	fieldSize := new(big.Int)
	fieldSize.SetString(fieldSizeStr, 10)
	fieldSizeSub1 := new(big.Int)
	fieldSizeSub1.SetString(fieldSizeSub1Str, 10)
	fieldSizeSub2 := new(big.Int).Sub(fieldSize, big.NewInt(2))

	edgeValue249bit := new(big.Int).Lsh(big.NewInt(1), 248)
	edgeValue248bit := new(big.Int).Sub(edgeValue249bit, big.NewInt(1))
	edgeValue248bitSubOne := new(big.Int).Sub(edgeValue248bit, big.NewInt(1))
	// Test cases
	testCases := []struct {
		a        *big.Int
		b        *big.Int
		expected bool
	}{
		{big.NewInt(2), big.NewInt(5), true},           // 2 < 5
		{big.NewInt(5), big.NewInt(2), false},          // 5 >= 2
		{big.NewInt(3), big.NewInt(3), false},          // 3 == 3
		{big.NewInt(0), big.NewInt(0), false},          // 0 == 0
		{big.NewInt(1), big.NewInt(1), false},          // 1 == 1
		{big.NewInt(0), big.NewInt(1), true},           // 0 < 1
		{big.NewInt(100), big.NewInt(1000), true},      // 100 < 1000
		{fieldSizeSub1, fieldSize, true},               // fieldSize - 1 < fieldSize
		{fieldSize, fieldSizeSub1, false},              // fieldSize < fieldSize - 1
		{fieldSize, fieldSizeSub2, false},              // fieldSize < fieldSize - 2
		{edgeValue248bit, edgeValue249bit, true},       // 2^248 - 1 < 2^248
		{edgeValue248bitSubOne, edgeValue248bit, true}, // 2^248 - 2 < 2^248 - 1
	}

	for _, tc := range testCases {
		var circuit IsLessCircuit
		if tc.expected {
			assert := test.NewAssert(t)
			assert.ProverSucceeded(&circuit, &IsLessCircuit{
				A: tc.a,
				B: tc.b,
			}, test.WithBackends(backend.GROTH16), test.WithCurves(ecc.BN254), test.NoSerialization())
		} else {
			assert := test.NewAssert(t)
			assert.ProverFailed(&circuit, &IsLessCircuit{
				A: tc.a,
				B: tc.b,
			}, test.WithBackends(backend.GROTH16), test.WithCurves(ecc.BN254), test.NoSerialization())
		}
	}
}
