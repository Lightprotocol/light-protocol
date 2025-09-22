package v1

import (
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
)

// TestCombinedCircuitManual tests the combined circuit without gnark's auto-generated test cases
func TestCombinedCircuitManual(t *testing.T) {
	// Test with 1 inclusion and 1 non-inclusion
	t.Run("1_1_ValidProof", func(t *testing.T) {
		inclusionAccounts := uint32(1)
		nonInclusionAccounts := uint32(1)
		treeHeight := uint32(26)

		params := BuildValidCombinedParameters(
			int(treeHeight), int(treeHeight),
			int(inclusionAccounts), int(nonInclusionAccounts))

		circuit := InitializeCombinedCircuit(
			treeHeight, inclusionAccounts,
			treeHeight, nonInclusionAccounts)

		// Fill inclusion data
		for i := 0; i < int(inclusionAccounts); i++ {
			circuit.Inclusion.Roots[i] = params.InclusionParameters.Inputs[i].Root
			circuit.Inclusion.Leaves[i] = params.InclusionParameters.Inputs[i].Leaf
			circuit.Inclusion.InPathIndices[i] = params.InclusionParameters.Inputs[i].PathIndex
			for j := 0; j < int(treeHeight); j++ {
				circuit.Inclusion.InPathElements[i][j] = params.InclusionParameters.Inputs[i].PathElements[j]
			}
		}

		// Fill non-inclusion data including NextIndices
		for i := 0; i < int(nonInclusionAccounts); i++ {
			circuit.NonInclusion.Roots[i] = params.NonInclusionParameters.Inputs[i].Root
			circuit.NonInclusion.Values[i] = params.NonInclusionParameters.Inputs[i].Value
			circuit.NonInclusion.LeafLowerRangeValues[i] = params.NonInclusionParameters.Inputs[i].LeafLowerRangeValue
			circuit.NonInclusion.LeafHigherRangeValues[i] = params.NonInclusionParameters.Inputs[i].LeafHigherRangeValue
			circuit.NonInclusion.NextIndices[i] = params.NonInclusionParameters.Inputs[i].NextIndex
			circuit.NonInclusion.InPathIndices[i] = params.NonInclusionParameters.Inputs[i].PathIndex
			for j := 0; j < int(treeHeight); j++ {
				circuit.NonInclusion.InPathElements[i][j] = params.NonInclusionParameters.Inputs[i].PathElements[j]
			}
		}

		// Compile the constraint system
		constraintCircuit := InitializeCombinedCircuit(
			treeHeight, inclusionAccounts,
			treeHeight, nonInclusionAccounts)

		ccs, err := frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &constraintCircuit)
		if err != nil {
			t.Fatalf("Failed to compile circuit: %v", err)
		}

		// Generate witness
		witness, err := frontend.NewWitness(&circuit, ecc.BN254.ScalarField())
		if err != nil {
			t.Fatalf("Failed to create witness: %v", err)
		}

		// Setup
		pk, vk, err := groth16.Setup(ccs)
		if err != nil {
			t.Fatalf("Failed to setup: %v", err)
		}

		// Prove
		proof, err := groth16.Prove(ccs, pk, witness)
		if err != nil {
			t.Fatalf("Failed to prove: %v", err)
		}

		// Verify
		publicWitness, err := witness.Public()
		if err != nil {
			t.Fatalf("Failed to get public witness: %v", err)
		}

		err = groth16.Verify(proof, vk, publicWitness)
		if err != nil {
			t.Fatalf("Failed to verify proof: %v", err)
		}
	})

	// Test with various combinations
	t.Run("VariousCombinations", func(t *testing.T) {
		testCases := []struct {
			name                 string
			inclusionAccounts    uint32
			nonInclusionAccounts uint32
		}{
			{"1_2", 1, 2},
			{"2_1", 2, 1},
			{"2_2", 2, 2},
			{"3_1", 3, 1},
			{"3_2", 3, 2},
			{"4_1", 4, 1},
			{"4_2", 4, 2},
		}

		for _, tc := range testCases {
			t.Run(tc.name, func(t *testing.T) {
				treeHeight := uint32(26)

				params := BuildValidCombinedParameters(
					int(treeHeight), int(treeHeight),
					int(tc.inclusionAccounts), int(tc.nonInclusionAccounts))

				circuit := InitializeCombinedCircuit(
					treeHeight, tc.inclusionAccounts,
					treeHeight, tc.nonInclusionAccounts)

				// Fill inclusion data
				for i := 0; i < int(tc.inclusionAccounts); i++ {
					circuit.Inclusion.Roots[i] = params.InclusionParameters.Inputs[i].Root
					circuit.Inclusion.Leaves[i] = params.InclusionParameters.Inputs[i].Leaf
					circuit.Inclusion.InPathIndices[i] = params.InclusionParameters.Inputs[i].PathIndex
					for j := 0; j < int(treeHeight); j++ {
						circuit.Inclusion.InPathElements[i][j] = params.InclusionParameters.Inputs[i].PathElements[j]
					}
				}

				// Fill non-inclusion data including NextIndices
				for i := 0; i < int(tc.nonInclusionAccounts); i++ {
					circuit.NonInclusion.Roots[i] = params.NonInclusionParameters.Inputs[i].Root
					circuit.NonInclusion.Values[i] = params.NonInclusionParameters.Inputs[i].Value
					circuit.NonInclusion.LeafLowerRangeValues[i] = params.NonInclusionParameters.Inputs[i].LeafLowerRangeValue
					circuit.NonInclusion.LeafHigherRangeValues[i] = params.NonInclusionParameters.Inputs[i].LeafHigherRangeValue
					circuit.NonInclusion.NextIndices[i] = params.NonInclusionParameters.Inputs[i].NextIndex
					circuit.NonInclusion.InPathIndices[i] = params.NonInclusionParameters.Inputs[i].PathIndex
					for j := 0; j < int(treeHeight); j++ {
						circuit.NonInclusion.InPathElements[i][j] = params.NonInclusionParameters.Inputs[i].PathElements[j]
					}
				}

				// Compile the constraint system
				constraintCircuit := InitializeCombinedCircuit(
					treeHeight, tc.inclusionAccounts,
					treeHeight, tc.nonInclusionAccounts)

				ccs, err := frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &constraintCircuit)
				if err != nil {
					t.Fatalf("Failed to compile circuit: %v", err)
				}

				// Generate witness
				witness, err := frontend.NewWitness(&circuit, ecc.BN254.ScalarField())
				if err != nil {
					t.Fatalf("Failed to create witness: %v", err)
				}

				// Setup (in production, this would be done once and keys saved)
				pk, vk, err := groth16.Setup(ccs)
				if err != nil {
					t.Fatalf("Failed to setup: %v", err)
				}

				// Prove
				proof, err := groth16.Prove(ccs, pk, witness)
				if err != nil {
					t.Fatalf("Failed to prove: %v", err)
				}

				// Verify
				publicWitness, err := witness.Public()
				if err != nil {
					t.Fatalf("Failed to get public witness: %v", err)
				}

				err = groth16.Verify(proof, vk, publicWitness)
				if err != nil {
					t.Fatalf("Failed to verify proof: %v", err)
				}
			})
		}
	})
}