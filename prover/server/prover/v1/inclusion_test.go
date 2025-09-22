package v1

import (
	"fmt"
	"light/light-prover/prover/common"
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/test"
	"github.com/reilabs/gnark-lean-extractor/v3/abstractor"
)

func TestInclusionCircuit(t *testing.T) {
	assert := test.NewAssert(t)

	// Test with single account
	t.Run("SingleAccount_ValidProof", func(t *testing.T) {
		numberOfAccounts := uint32(1)
		treeHeight := uint32(26)

		// Create test parameters
		params := BuildTestTree(int(treeHeight), int(numberOfAccounts), false)

		// Setup circuit structure
		var circuit InclusionCircuit
		circuit.Roots = make([]frontend.Variable, numberOfAccounts)
		circuit.Leaves = make([]frontend.Variable, numberOfAccounts)
		circuit.InPathIndices = make([]frontend.Variable, numberOfAccounts)
		circuit.InPathElements = make([][]frontend.Variable, numberOfAccounts)
		for i := 0; i < int(numberOfAccounts); i++ {
			circuit.InPathElements[i] = make([]frontend.Variable, treeHeight)
		}
		circuit.NumberOfCompressedAccounts = numberOfAccounts
		circuit.Height = treeHeight

		// Create witness
		roots := make([]frontend.Variable, numberOfAccounts)
		leaves := make([]frontend.Variable, numberOfAccounts)
		inPathIndices := make([]frontend.Variable, numberOfAccounts)
		inPathElements := make([][]frontend.Variable, numberOfAccounts)

		for i := 0; i < int(numberOfAccounts); i++ {
			roots[i] = params.Inputs[i].Root
			leaves[i] = params.Inputs[i].Leaf
			inPathIndices[i] = params.Inputs[i].PathIndex
			inPathElements[i] = make([]frontend.Variable, treeHeight)
			for j := 0; j < int(treeHeight); j++ {
				inPathElements[i][j] = params.Inputs[i].PathElements[j]
			}
		}

		witness := &InclusionCircuit{
			Roots:                      roots,
			Leaves:                     leaves,
			InPathIndices:              inPathIndices,
			InPathElements:             inPathElements,
			NumberOfCompressedAccounts: numberOfAccounts,
			Height:                     treeHeight,
		}

		assert.ProverSucceeded(&circuit, witness,
			test.WithBackends(backend.GROTH16),
			test.WithCurves(ecc.BN254),
			test.NoSerializationChecks())
	})

	// Test with multiple accounts
	t.Run("MultipleAccounts_ValidProof", func(t *testing.T) {
		testCases := []uint32{2, 3, 4}

		for _, numberOfAccounts := range testCases {
			t.Run(fmt.Sprintf("%d_accounts", numberOfAccounts), func(t *testing.T) {
				treeHeight := uint32(26)

				params := BuildTestTree(int(treeHeight), int(numberOfAccounts), true)

				var circuit InclusionCircuit
				circuit.Roots = make([]frontend.Variable, numberOfAccounts)
				circuit.Leaves = make([]frontend.Variable, numberOfAccounts)
				circuit.InPathIndices = make([]frontend.Variable, numberOfAccounts)
				circuit.InPathElements = make([][]frontend.Variable, numberOfAccounts)
				for i := 0; i < int(numberOfAccounts); i++ {
					circuit.InPathElements[i] = make([]frontend.Variable, treeHeight)
				}
				circuit.NumberOfCompressedAccounts = numberOfAccounts
				circuit.Height = treeHeight

				roots := make([]frontend.Variable, numberOfAccounts)
				leaves := make([]frontend.Variable, numberOfAccounts)
				inPathIndices := make([]frontend.Variable, numberOfAccounts)
				inPathElements := make([][]frontend.Variable, numberOfAccounts)

				for i := 0; i < int(numberOfAccounts); i++ {
					roots[i] = params.Inputs[i].Root
					leaves[i] = params.Inputs[i].Leaf
					inPathIndices[i] = params.Inputs[i].PathIndex
					inPathElements[i] = make([]frontend.Variable, treeHeight)
					for j := 0; j < int(treeHeight); j++ {
						inPathElements[i][j] = params.Inputs[i].PathElements[j]
					}
				}

				witness := &InclusionCircuit{
					Roots:                      roots,
					Leaves:                     leaves,
					InPathIndices:              inPathIndices,
					InPathElements:             inPathElements,
					NumberOfCompressedAccounts: numberOfAccounts,
					Height:                     treeHeight,
				}

				assert.ProverSucceeded(&circuit, witness,
					test.WithBackends(backend.GROTH16),
					test.WithCurves(ecc.BN254),
					test.NoSerializationChecks())
			})
		}
	})

	// Test invalid proof (wrong root)
	t.Run("SingleAccount_InvalidRoot", func(t *testing.T) {
		numberOfAccounts := uint32(1)
		treeHeight := uint32(26)

		params := BuildTestTree(int(treeHeight), int(numberOfAccounts), false)

		var circuit InclusionCircuit
		circuit.Roots = make([]frontend.Variable, numberOfAccounts)
		circuit.Leaves = make([]frontend.Variable, numberOfAccounts)
		circuit.InPathIndices = make([]frontend.Variable, numberOfAccounts)
		circuit.InPathElements = make([][]frontend.Variable, numberOfAccounts)
		for i := 0; i < int(numberOfAccounts); i++ {
			circuit.InPathElements[i] = make([]frontend.Variable, treeHeight)
		}
		circuit.NumberOfCompressedAccounts = numberOfAccounts
		circuit.Height = treeHeight

		// Create witness with wrong root
		roots := make([]frontend.Variable, numberOfAccounts)
		leaves := make([]frontend.Variable, numberOfAccounts)
		inPathIndices := make([]frontend.Variable, numberOfAccounts)
		inPathElements := make([][]frontend.Variable, numberOfAccounts)

		for i := 0; i < int(numberOfAccounts); i++ {
			roots[i] = 12345 // Wrong root
			leaves[i] = params.Inputs[i].Leaf
			inPathIndices[i] = params.Inputs[i].PathIndex
			inPathElements[i] = make([]frontend.Variable, treeHeight)
			for j := 0; j < int(treeHeight); j++ {
				inPathElements[i][j] = params.Inputs[i].PathElements[j]
			}
		}

		witness := &InclusionCircuit{
			Roots:                      roots,
			Leaves:                     leaves,
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
}

// TestInclusionGadget tests the InclusionProof gadget directly
func TestInclusionGadget(t *testing.T) {
	assert := test.NewAssert(t)

	numberOfAccounts := uint32(1)
	treeHeight := uint32(26)
	params := BuildTestTree(int(treeHeight), int(numberOfAccounts), false)

	// Create a simple circuit that just uses the gadget
	var circuit testInclusionGadgetCircuit
	circuit.Roots = make([]frontend.Variable, numberOfAccounts)
	circuit.Leaves = make([]frontend.Variable, numberOfAccounts)
	circuit.InPathIndices = make([]frontend.Variable, numberOfAccounts)
	circuit.InPathElements = make([][]frontend.Variable, numberOfAccounts)
	circuit.InPathElements[0] = make([]frontend.Variable, treeHeight)
	circuit.NumberOfCompressedAccounts = numberOfAccounts
	circuit.Height = treeHeight

	pathElements := make([]frontend.Variable, treeHeight)
	for i := 0; i < int(treeHeight); i++ {
		pathElements[i] = params.Inputs[0].PathElements[i]
	}
	witness := &testInclusionGadgetCircuit{
		Roots:                      []frontend.Variable{params.Inputs[0].Root},
		Leaves:                     []frontend.Variable{params.Inputs[0].Leaf},
		InPathIndices:              []frontend.Variable{params.Inputs[0].PathIndex},
		InPathElements:             [][]frontend.Variable{pathElements},
		NumberOfCompressedAccounts: numberOfAccounts,
		Height:                     treeHeight,
	}

	assert.ProverSucceeded(&circuit, witness,
		test.WithBackends(backend.GROTH16),
		test.WithCurves(ecc.BN254),
		test.NoSerializationChecks())
}

// Test circuit for gadget testing
type testInclusionGadgetCircuit struct {
	Roots  []frontend.Variable `gnark:",public"`
	Leaves []frontend.Variable `gnark:",public"`

	InPathIndices  []frontend.Variable
	InPathElements [][]frontend.Variable

	NumberOfCompressedAccounts uint32
	Height                     uint32
}

func (circuit *testInclusionGadgetCircuit) Define(api frontend.API) error {
	abstractor.CallVoid(api, common.InclusionProof{
		Roots:                      circuit.Roots,
		Leaves:                     circuit.Leaves,
		InPathElements:             circuit.InPathElements,
		InPathIndices:              circuit.InPathIndices,
		NumberOfCompressedAccounts: circuit.NumberOfCompressedAccounts,
		Height:                     circuit.Height,
	})
	return nil
}