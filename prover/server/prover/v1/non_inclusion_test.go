package v1

import (
	"math/big"
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/test"
)

func TestNonInclusionCircuit(t *testing.T) {
	assert := test.NewAssert(t)

	// Test with single account
	t.Run("SingleAccount_ValidProof", func(t *testing.T) {
		numberOfAccounts := uint32(1)
		treeHeight := uint32(26)

		// Create test parameters
		params := BuildValidTestNonInclusionTree(int(treeHeight), int(numberOfAccounts), false)

		// Setup circuit structure
		var circuit NonInclusionCircuit
		circuit.Roots = make([]frontend.Variable, numberOfAccounts)
		circuit.Values = make([]frontend.Variable, numberOfAccounts)
		circuit.LeafLowerRangeValues = make([]frontend.Variable, numberOfAccounts)
		circuit.LeafHigherRangeValues = make([]frontend.Variable, numberOfAccounts)
		circuit.NextIndices = make([]frontend.Variable, numberOfAccounts)
		circuit.InPathIndices = make([]frontend.Variable, numberOfAccounts)
		circuit.InPathElements = make([][]frontend.Variable, numberOfAccounts)
		for i := 0; i < int(numberOfAccounts); i++ {
			circuit.InPathElements[i] = make([]frontend.Variable, treeHeight)
		}
		circuit.NumberOfCompressedAccounts = numberOfAccounts
		circuit.Height = treeHeight

		// Create witness
		roots := make([]frontend.Variable, numberOfAccounts)
		values := make([]frontend.Variable, numberOfAccounts)
		leafLowerRangeValues := make([]frontend.Variable, numberOfAccounts)
		leafHigherRangeValues := make([]frontend.Variable, numberOfAccounts)
		nextIndices := make([]frontend.Variable, numberOfAccounts)
		inPathIndices := make([]frontend.Variable, numberOfAccounts)
		inPathElements := make([][]frontend.Variable, numberOfAccounts)

		for i := 0; i < int(numberOfAccounts); i++ {
			roots[i] = params.Inputs[i].Root
			values[i] = params.Inputs[i].Value
			leafLowerRangeValues[i] = params.Inputs[i].LeafLowerRangeValue
			leafHigherRangeValues[i] = params.Inputs[i].LeafHigherRangeValue
			nextIndices[i] = params.Inputs[i].NextIndex
			inPathIndices[i] = params.Inputs[i].PathIndex
			inPathElements[i] = make([]frontend.Variable, treeHeight)
			for j := 0; j < int(treeHeight); j++ {
				inPathElements[i][j] = params.Inputs[i].PathElements[j]
			}
		}

		witness := &NonInclusionCircuit{
			Roots:                      roots,
			Values:                     values,
			LeafLowerRangeValues:       leafLowerRangeValues,
			LeafHigherRangeValues:      leafHigherRangeValues,
			NextIndices:                nextIndices,
			InPathIndices:              inPathIndices,
			InPathElements:             inPathElements,
			NumberOfCompressedAccounts: numberOfAccounts,
			Height:                     treeHeight,
		}

		assert.ProverSucceeded(&circuit, witness,
			test.WithBackends(backend.GROTH16),
			test.WithCurves(ecc.BN254),
			test.NoSerializationChecks(),
			test.NoTestEngine())
	})

	// Test with multiple accounts
	t.Run("TwoAccounts_ValidProof", func(t *testing.T) {
		numberOfAccounts := uint32(2)
		treeHeight := uint32(26)

		params := BuildValidTestNonInclusionTree(int(treeHeight), int(numberOfAccounts), true)

		var circuit NonInclusionCircuit
		circuit.Roots = make([]frontend.Variable, numberOfAccounts)
		circuit.Values = make([]frontend.Variable, numberOfAccounts)
		circuit.LeafLowerRangeValues = make([]frontend.Variable, numberOfAccounts)
		circuit.LeafHigherRangeValues = make([]frontend.Variable, numberOfAccounts)
		circuit.NextIndices = make([]frontend.Variable, numberOfAccounts)
		circuit.InPathIndices = make([]frontend.Variable, numberOfAccounts)
		circuit.InPathElements = make([][]frontend.Variable, numberOfAccounts)
		for i := 0; i < int(numberOfAccounts); i++ {
			circuit.InPathElements[i] = make([]frontend.Variable, treeHeight)
		}
		circuit.NumberOfCompressedAccounts = numberOfAccounts
		circuit.Height = treeHeight

		roots := make([]frontend.Variable, numberOfAccounts)
		values := make([]frontend.Variable, numberOfAccounts)
		leafLowerRangeValues := make([]frontend.Variable, numberOfAccounts)
		leafHigherRangeValues := make([]frontend.Variable, numberOfAccounts)
		nextIndices := make([]frontend.Variable, numberOfAccounts)
		inPathIndices := make([]frontend.Variable, numberOfAccounts)
		inPathElements := make([][]frontend.Variable, numberOfAccounts)

		for i := 0; i < int(numberOfAccounts); i++ {
			roots[i] = params.Inputs[i].Root
			values[i] = params.Inputs[i].Value
			leafLowerRangeValues[i] = params.Inputs[i].LeafLowerRangeValue
			leafHigherRangeValues[i] = params.Inputs[i].LeafHigherRangeValue
			nextIndices[i] = params.Inputs[i].NextIndex
			inPathIndices[i] = params.Inputs[i].PathIndex
			inPathElements[i] = make([]frontend.Variable, treeHeight)
			for j := 0; j < int(treeHeight); j++ {
				inPathElements[i][j] = params.Inputs[i].PathElements[j]
			}
		}

		witness := &NonInclusionCircuit{
			Roots:                      roots,
			Values:                     values,
			LeafLowerRangeValues:       leafLowerRangeValues,
			LeafHigherRangeValues:      leafHigherRangeValues,
			NextIndices:                nextIndices,
			InPathIndices:              inPathIndices,
			InPathElements:             inPathElements,
			NumberOfCompressedAccounts: numberOfAccounts,
			Height:                     treeHeight,
		}

		assert.ProverSucceeded(&circuit, witness,
			test.WithBackends(backend.GROTH16),
			test.WithCurves(ecc.BN254),
			test.NoSerializationChecks(),
			test.NoTestEngine())
	})

	// Test invalid proof (value out of range)
	t.Run("SingleAccount_InvalidProof_ValueTooLow", func(t *testing.T) {
		numberOfAccounts := uint32(1)
		treeHeight := uint32(26)

		// Build invalid tree with value too low
		params := BuildTestNonInclusionTree(int(treeHeight), int(numberOfAccounts), false, false, true)

		var circuit NonInclusionCircuit
		circuit.Roots = make([]frontend.Variable, numberOfAccounts)
		circuit.Values = make([]frontend.Variable, numberOfAccounts)
		circuit.LeafLowerRangeValues = make([]frontend.Variable, numberOfAccounts)
		circuit.LeafHigherRangeValues = make([]frontend.Variable, numberOfAccounts)
		circuit.NextIndices = make([]frontend.Variable, numberOfAccounts)
		circuit.InPathIndices = make([]frontend.Variable, numberOfAccounts)
		circuit.InPathElements = make([][]frontend.Variable, numberOfAccounts)
		for i := 0; i < int(numberOfAccounts); i++ {
			circuit.InPathElements[i] = make([]frontend.Variable, treeHeight)
		}
		circuit.NumberOfCompressedAccounts = numberOfAccounts
		circuit.Height = treeHeight

		roots := make([]frontend.Variable, numberOfAccounts)
		values := make([]frontend.Variable, numberOfAccounts)
		leafLowerRangeValues := make([]frontend.Variable, numberOfAccounts)
		leafHigherRangeValues := make([]frontend.Variable, numberOfAccounts)
		nextIndices := make([]frontend.Variable, numberOfAccounts)
		inPathIndices := make([]frontend.Variable, numberOfAccounts)
		inPathElements := make([][]frontend.Variable, numberOfAccounts)

		for i := 0; i < int(numberOfAccounts); i++ {
			roots[i] = params.Inputs[i].Root
			values[i] = params.Inputs[i].Value
			leafLowerRangeValues[i] = params.Inputs[i].LeafLowerRangeValue
			leafHigherRangeValues[i] = params.Inputs[i].LeafHigherRangeValue
			nextIndices[i] = params.Inputs[i].NextIndex
			inPathIndices[i] = params.Inputs[i].PathIndex
			inPathElements[i] = make([]frontend.Variable, treeHeight)
			for j := 0; j < int(treeHeight); j++ {
				inPathElements[i][j] = params.Inputs[i].PathElements[j]
			}
		}

		witness := &NonInclusionCircuit{
			Roots:                      roots,
			Values:                     values,
			LeafLowerRangeValues:       leafLowerRangeValues,
			LeafHigherRangeValues:      leafHigherRangeValues,
			NextIndices:                nextIndices,
			InPathIndices:              inPathIndices,
			InPathElements:             inPathElements,
			NumberOfCompressedAccounts: numberOfAccounts,
			Height:                     treeHeight,
		}

		assert.ProverFailed(&circuit, witness,
			test.WithBackends(backend.GROTH16),
			test.WithCurves(ecc.BN254),
			test.NoSerializationChecks())
	})

	// Test with maximum field value (HIGHEST_ADDRESS_PLUS_ONE scenario)
	t.Run("SingleAccount_MaxFieldValue", func(t *testing.T) {
		numberOfAccounts := uint32(1)
		treeHeight := uint32(26)

		// Create parameters with max field value
		params := NonInclusionParameters{
			Inputs: make([]NonInclusionInputs, numberOfAccounts),
		}

		// Set up the indexed merkle tree scenario with max value
		leafLowerRangeValue := big.NewInt(0)
		leafHigherRangeValue := new(big.Int)
		leafHigherRangeValue.SetString("00ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff", 16)
		value := new(big.Int).SetInt64(100) // Some value between 0 and max

		// Build a merkle tree and get the proof
		tree := BuildIndexedMerkleTree(int(treeHeight))
		pathElements, pathIndex := tree.GenerateProof(0) // Get proof for index 0
		root := tree.Root()

		params.Inputs[0] = NonInclusionInputs{
			Root:                 *root,
			Value:                *value,
			LeafLowerRangeValue:  *leafLowerRangeValue,
			LeafHigherRangeValue: *leafHigherRangeValue,
			NextIndex:            uint32(0),
			PathIndex:            uint32(pathIndex),
			PathElements:         pathElements,
		}

		var circuit NonInclusionCircuit
		circuit.Roots = make([]frontend.Variable, numberOfAccounts)
		circuit.Values = make([]frontend.Variable, numberOfAccounts)
		circuit.LeafLowerRangeValues = make([]frontend.Variable, numberOfAccounts)
		circuit.LeafHigherRangeValues = make([]frontend.Variable, numberOfAccounts)
		circuit.NextIndices = make([]frontend.Variable, numberOfAccounts)
		circuit.InPathIndices = make([]frontend.Variable, numberOfAccounts)
		circuit.InPathElements = make([][]frontend.Variable, numberOfAccounts)
		circuit.InPathElements[0] = make([]frontend.Variable, treeHeight)
		circuit.NumberOfCompressedAccounts = numberOfAccounts
		circuit.Height = treeHeight

		witnessPathElements := make([]frontend.Variable, treeHeight)
		for i := 0; i < int(treeHeight); i++ {
			witnessPathElements[i] = params.Inputs[0].PathElements[i]
		}
		witness := &NonInclusionCircuit{
			Roots:                      []frontend.Variable{params.Inputs[0].Root},
			Values:                     []frontend.Variable{params.Inputs[0].Value},
			LeafLowerRangeValues:       []frontend.Variable{params.Inputs[0].LeafLowerRangeValue},
			LeafHigherRangeValues:      []frontend.Variable{params.Inputs[0].LeafHigherRangeValue},
			NextIndices:                []frontend.Variable{params.Inputs[0].NextIndex},
			InPathIndices:              []frontend.Variable{params.Inputs[0].PathIndex},
			InPathElements:             [][]frontend.Variable{witnessPathElements},
			NumberOfCompressedAccounts: numberOfAccounts,
			Height:                     treeHeight,
		}

		assert.ProverSucceeded(&circuit, witness,
			test.WithBackends(backend.GROTH16),
			test.WithCurves(ecc.BN254),
			test.NoSerializationChecks(),
			test.NoTestEngine())
	})
}

// TestNonInclusionGadget tests the NonInclusionProof gadget directly
func TestNonInclusionGadget(t *testing.T) {
	assert := test.NewAssert(t)

	numberOfAccounts := uint32(1)
	treeHeight := uint32(26)
	params := BuildValidTestNonInclusionTree(int(treeHeight), int(numberOfAccounts), false)

	// Create a simple circuit that just uses the gadget
	var circuit testNonInclusionGadgetCircuit
	circuit.Roots = make([]frontend.Variable, numberOfAccounts)
	circuit.Values = make([]frontend.Variable, numberOfAccounts)
	circuit.LeafLowerRangeValues = make([]frontend.Variable, numberOfAccounts)
	circuit.LeafHigherRangeValues = make([]frontend.Variable, numberOfAccounts)
	circuit.NextIndices = make([]frontend.Variable, numberOfAccounts)
	circuit.InPathIndices = make([]frontend.Variable, numberOfAccounts)
	circuit.InPathElements = make([][]frontend.Variable, numberOfAccounts)
	circuit.InPathElements[0] = make([]frontend.Variable, treeHeight)
	circuit.NumberOfCompressedAccounts = numberOfAccounts
	circuit.Height = treeHeight

	pathElements := make([]frontend.Variable, treeHeight)
	for i := 0; i < int(treeHeight); i++ {
		pathElements[i] = params.Inputs[0].PathElements[i]
	}
	witness := &testNonInclusionGadgetCircuit{
		Roots:                      []frontend.Variable{params.Inputs[0].Root},
		Values:                     []frontend.Variable{params.Inputs[0].Value},
		LeafLowerRangeValues:       []frontend.Variable{params.Inputs[0].LeafLowerRangeValue},
		LeafHigherRangeValues:      []frontend.Variable{params.Inputs[0].LeafHigherRangeValue},
		NextIndices:                []frontend.Variable{params.Inputs[0].NextIndex},
		InPathIndices:              []frontend.Variable{params.Inputs[0].PathIndex},
		InPathElements:             [][]frontend.Variable{pathElements},
		NumberOfCompressedAccounts: numberOfAccounts,
		Height:                     treeHeight,
	}

	assert.ProverSucceeded(&circuit, witness,
		test.WithBackends(backend.GROTH16),
		test.WithCurves(ecc.BN254),
		test.NoSerializationChecks(),
		test.NoTestEngine())
}

// Test circuit for gadget testing
type testNonInclusionGadgetCircuit struct {
	Roots  []frontend.Variable `gnark:",public"`
	Values []frontend.Variable `gnark:",public"`

	LeafLowerRangeValues  []frontend.Variable
	LeafHigherRangeValues []frontend.Variable
	NextIndices            []frontend.Variable

	InPathIndices  []frontend.Variable
	InPathElements [][]frontend.Variable

	NumberOfCompressedAccounts uint32
	Height                     uint32
}

func (circuit *testNonInclusionGadgetCircuit) Define(api frontend.API) error {
	proof := LegacyNonInclusionProof{
		Roots:                      circuit.Roots,
		Values:                     circuit.Values,
		LeafLowerRangeValues:       circuit.LeafLowerRangeValues,
		LeafHigherRangeValues:      circuit.LeafHigherRangeValues,
		NextIndices:                circuit.NextIndices,
		InPathElements:             circuit.InPathElements,
		InPathIndices:              circuit.InPathIndices,
		NumberOfCompressedAccounts: circuit.NumberOfCompressedAccounts,
		Height:                     circuit.Height,
	}
	_ = proof.DefineGadget(api)
	return nil
}
