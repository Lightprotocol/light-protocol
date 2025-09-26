package v1

import (
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend"
	"github.com/consensys/gnark/test"
)

func TestCombinedCircuit(t *testing.T) {
	assert := test.NewAssert(t)

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

		// Fill non-inclusion data
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

		constraintCircuit := InitializeCombinedCircuit(
			treeHeight, inclusionAccounts,
			treeHeight, nonInclusionAccounts)
		assert.ProverSucceeded(&constraintCircuit, &circuit,
			test.WithBackends(backend.GROTH16),
			test.WithCurves(ecc.BN254),
			test.NoSerializationChecks(),
			test.NoTestEngine())
	})

	// Test with 2 inclusions and 1 non-inclusion
	t.Run("2_1_ValidProof", func(t *testing.T) {
		inclusionAccounts := uint32(2)
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

		// Fill non-inclusion data
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

		constraintCircuit := InitializeCombinedCircuit(
			treeHeight, inclusionAccounts,
			treeHeight, nonInclusionAccounts)
		assert.ProverSucceeded(&constraintCircuit, &circuit,
			test.WithBackends(backend.GROTH16),
			test.WithCurves(ecc.BN254),
			test.NoSerializationChecks(),
			test.NoTestEngine())
	})

	// Test various combinations
	t.Run("VariousCombinations", func(t *testing.T) {
		testCases := []struct {
			name                 string
			inclusionAccounts    uint32
			nonInclusionAccounts uint32
		}{
			{"1_2", 1, 2},
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

				// Fill non-inclusion data
				for i := 0; i < int(tc.nonInclusionAccounts); i++ {
					circuit.NonInclusion.Roots[i] = params.NonInclusionParameters.Inputs[i].Root
					circuit.NonInclusion.Values[i] = params.NonInclusionParameters.Inputs[i].Value
					circuit.NonInclusion.LeafLowerRangeValues[i] = params.NonInclusionParameters.Inputs[i].LeafLowerRangeValue
					circuit.NonInclusion.LeafHigherRangeValues[i] = params.NonInclusionParameters.Inputs[i].LeafHigherRangeValue
					circuit.NonInclusion.InPathIndices[i] = params.NonInclusionParameters.Inputs[i].PathIndex
					for j := 0; j < int(treeHeight); j++ {
						circuit.NonInclusion.InPathElements[i][j] = params.NonInclusionParameters.Inputs[i].PathElements[j]
					}
				}

				constraintCircuit := InitializeCombinedCircuit(
					treeHeight, tc.inclusionAccounts,
					treeHeight, tc.nonInclusionAccounts)
				assert.ProverSucceeded(&constraintCircuit, &circuit,
					test.WithBackends(backend.GROTH16),
					test.WithCurves(ecc.BN254),
					test.NoSerializationChecks())
			})
		}
	})

	// Test invalid proof (wrong inclusion root)
	t.Run("1_1_InvalidInclusionRoot", func(t *testing.T) {
		inclusionAccounts := uint32(1)
		nonInclusionAccounts := uint32(1)
		treeHeight := uint32(26)

		params := BuildValidCombinedParameters(
			int(treeHeight), int(treeHeight),
			int(inclusionAccounts), int(nonInclusionAccounts))

		circuit := InitializeCombinedCircuit(
			treeHeight, inclusionAccounts,
			treeHeight, nonInclusionAccounts)

		// Fill inclusion data with wrong root
		circuit.Inclusion.Roots[0] = 12345 // Wrong root
		circuit.Inclusion.Leaves[0] = params.InclusionParameters.Inputs[0].Leaf
		circuit.Inclusion.InPathIndices[0] = params.InclusionParameters.Inputs[0].PathIndex
		for j := 0; j < int(treeHeight); j++ {
			circuit.Inclusion.InPathElements[0][j] = params.InclusionParameters.Inputs[0].PathElements[j]
		}

		// Fill non-inclusion data correctly
		circuit.NonInclusion.Roots[0] = params.NonInclusionParameters.Inputs[0].Root
		circuit.NonInclusion.Values[0] = params.NonInclusionParameters.Inputs[0].Value
		circuit.NonInclusion.LeafLowerRangeValues[0] = params.NonInclusionParameters.Inputs[0].LeafLowerRangeValue
		circuit.NonInclusion.LeafHigherRangeValues[0] = params.NonInclusionParameters.Inputs[0].LeafHigherRangeValue
		circuit.NonInclusion.InPathIndices[0] = params.NonInclusionParameters.Inputs[0].PathIndex
		for j := 0; j < int(treeHeight); j++ {
			circuit.NonInclusion.InPathElements[0][j] = params.NonInclusionParameters.Inputs[0].PathElements[j]
		}

		constraintCircuit := InitializeCombinedCircuit(
			treeHeight, inclusionAccounts,
			treeHeight, nonInclusionAccounts)
		assert.ProverFailed(&constraintCircuit, &circuit,
			test.WithBackends(backend.GROTH16),
			test.WithCurves(ecc.BN254),
			test.NoSerializationChecks(),
			test.NoTestEngine())
	})

	// Test invalid proof (value out of range for non-inclusion)
	t.Run("1_1_InvalidNonInclusionValue", func(t *testing.T) {
		inclusionAccounts := uint32(1)
		nonInclusionAccounts := uint32(1)
		treeHeight := uint32(26)

		inclusionParams := BuildTestTree(int(treeHeight), int(inclusionAccounts), false)
		// Build invalid non-inclusion with value too low
		nonInclusionParams := BuildTestNonInclusionTree(int(treeHeight), int(nonInclusionAccounts), false, false, true)

		circuit := InitializeCombinedCircuit(
			treeHeight, inclusionAccounts,
			treeHeight, nonInclusionAccounts)

		// Fill inclusion data
		circuit.Inclusion.Roots[0] = inclusionParams.Inputs[0].Root
		circuit.Inclusion.Leaves[0] = inclusionParams.Inputs[0].Leaf
		circuit.Inclusion.InPathIndices[0] = inclusionParams.Inputs[0].PathIndex
		for j := 0; j < int(treeHeight); j++ {
			circuit.Inclusion.InPathElements[0][j] = inclusionParams.Inputs[0].PathElements[j]
		}

		// Fill non-inclusion data with invalid value
		circuit.NonInclusion.Roots[0] = nonInclusionParams.Inputs[0].Root
		circuit.NonInclusion.Values[0] = nonInclusionParams.Inputs[0].Value
		circuit.NonInclusion.LeafLowerRangeValues[0] = nonInclusionParams.Inputs[0].LeafLowerRangeValue
		circuit.NonInclusion.LeafHigherRangeValues[0] = nonInclusionParams.Inputs[0].LeafHigherRangeValue
		circuit.NonInclusion.InPathIndices[0] = nonInclusionParams.Inputs[0].PathIndex
		for j := 0; j < int(treeHeight); j++ {
			circuit.NonInclusion.InPathElements[0][j] = nonInclusionParams.Inputs[0].PathElements[j]
		}

		constraintCircuit := InitializeCombinedCircuit(
			treeHeight, inclusionAccounts,
			treeHeight, nonInclusionAccounts)
		assert.ProverFailed(&constraintCircuit, &circuit,
			test.WithBackends(backend.GROTH16),
			test.WithCurves(ecc.BN254),
			test.NoSerializationChecks(),
			test.NoTestEngine())
	})
}