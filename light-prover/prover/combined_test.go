package prover

import (
	"bufio"
	"encoding/json"
	"fmt"
	"os"
	"strings"
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/test"
)

func TestCombined(t *testing.T) {
	assert := test.NewAssert(t)

	file, err := os.Open("../test-data/combined.csv")
	defer func(file *os.File) {
		err := file.Close()
		if err != nil {
			fmt.Println("Error closing file: ", err)
		}
	}(file)

	assert.Nil(err, "Error opening file: ", err)

	scanner := bufio.NewScanner(file)
	for scanner.Scan() {
		line := scanner.Text()
		if line == "" {
			continue
		}
		splitLine := strings.Split(line, ";")
		assert.Equal(len(splitLine), 2, "Invalid line: ", line)

		var params CombinedParameters
		err := json.Unmarshal([]byte(splitLine[1]), &params)
		assert.Nil(err, "Error unmarshalling inputs: ", err)

		assert.NotEqual(len(params.InclusionParameters.Inputs), 0)
		assert.NotEqual(len(params.NonInclusionParameters.Inputs), 0)

		var inclusionNumberOfCompressedAccounts = params.InclusionParameters.NumberOfCompressedAccounts()
		var inclusionTreeDepth = params.InclusionParameters.TreeDepth()

		inclusionRoots := make([]frontend.Variable, inclusionNumberOfCompressedAccounts)
		inclusionLeaves := make([]frontend.Variable, inclusionNumberOfCompressedAccounts)
		inclusionInPathIndices := make([]frontend.Variable, inclusionNumberOfCompressedAccounts)
		inclusionInPathElements := make([][]frontend.Variable, inclusionNumberOfCompressedAccounts)
		for i := 0; i < int(inclusionNumberOfCompressedAccounts); i++ {
			inclusionInPathElements[i] = make([]frontend.Variable, inclusionTreeDepth)
		}

		for i, v := range params.InclusionParameters.Inputs {
			inclusionRoots[i] = v.Root
			inclusionLeaves[i] = v.Leaf
			inclusionInPathIndices[i] = v.PathIndex
			for j, v2 := range v.PathElements {
				inclusionInPathElements[i][j] = v2
			}
		}

		var nonInclusionNumberOfCompressedAccounts = params.NonInclusionParameters.NumberOfCompressedAccounts()
		var nonInclusionTreeDepth = params.NonInclusionParameters.TreeDepth()

		nonInclusionRoots := make([]frontend.Variable, nonInclusionNumberOfCompressedAccounts)
		nonInclusionValues := make([]frontend.Variable, nonInclusionNumberOfCompressedAccounts)
		nonInclusionLeafLowerRangeValues := make([]frontend.Variable, nonInclusionNumberOfCompressedAccounts)
		nonInclusionLeafHigherRangeValues := make([]frontend.Variable, nonInclusionNumberOfCompressedAccounts)
		nonInclusionLeafIndices := make([]frontend.Variable, nonInclusionNumberOfCompressedAccounts)
		nonInclusionInPathIndices := make([]frontend.Variable, nonInclusionNumberOfCompressedAccounts)
		nonInclusionInPathElements := make([][]frontend.Variable, nonInclusionNumberOfCompressedAccounts)
		for i := 0; i < int(nonInclusionNumberOfCompressedAccounts); i++ {
			nonInclusionInPathElements[i] = make([]frontend.Variable, nonInclusionTreeDepth)
		}

		for i, v := range params.NonInclusionParameters.Inputs {
			nonInclusionRoots[i] = v.Root
			nonInclusionValues[i] = v.Value
			nonInclusionLeafLowerRangeValues[i] = v.LeafLowerRangeValue
			nonInclusionLeafHigherRangeValues[i] = v.LeafHigherRangeValue
			nonInclusionLeafIndices[i] = v.NextIndex
			nonInclusionInPathIndices[i] = v.PathIndex
			for j, v2 := range v.PathElements {
				nonInclusionInPathElements[i][j] = v2
			}
		}

		var circuit CombinedCircuit
		circuit.Inclusion = InclusionCircuit{}
		circuit.NonInclusion = NonInclusionCircuit{}

		circuit.Inclusion.Roots = make([]frontend.Variable, inclusionNumberOfCompressedAccounts)
		circuit.Inclusion.Leaves = make([]frontend.Variable, inclusionNumberOfCompressedAccounts)
		circuit.Inclusion.InPathIndices = make([]frontend.Variable, inclusionNumberOfCompressedAccounts)
		circuit.Inclusion.InPathElements = make([][]frontend.Variable, inclusionNumberOfCompressedAccounts)
		for i := 0; i < int(inclusionNumberOfCompressedAccounts); i++ {
			circuit.Inclusion.InPathElements[i] = make([]frontend.Variable, inclusionTreeDepth)
		}

		circuit.Inclusion.NumberOfCompressedAccounts = inclusionNumberOfCompressedAccounts
		circuit.Inclusion.Depth = inclusionTreeDepth

		circuit.NonInclusion.Roots = make([]frontend.Variable, nonInclusionNumberOfCompressedAccounts)
		circuit.NonInclusion.Values = make([]frontend.Variable, nonInclusionNumberOfCompressedAccounts)
		circuit.NonInclusion.LeafLowerRangeValues = make([]frontend.Variable, nonInclusionNumberOfCompressedAccounts)
		circuit.NonInclusion.LeafHigherRangeValues = make([]frontend.Variable, nonInclusionNumberOfCompressedAccounts)
		circuit.NonInclusion.NextIndices = make([]frontend.Variable, nonInclusionNumberOfCompressedAccounts)
		circuit.NonInclusion.InPathIndices = make([]frontend.Variable, nonInclusionNumberOfCompressedAccounts)
		circuit.NonInclusion.InPathElements = make([][]frontend.Variable, nonInclusionNumberOfCompressedAccounts)
		for i := 0; i < int(nonInclusionNumberOfCompressedAccounts); i++ {
			circuit.NonInclusion.InPathElements[i] = make([]frontend.Variable, nonInclusionTreeDepth)
		}

		circuit.NonInclusion.NumberOfCompressedAccounts = nonInclusionNumberOfCompressedAccounts
		circuit.NonInclusion.Depth = nonInclusionTreeDepth

		assignment := &CombinedCircuit{
			Inclusion: InclusionCircuit{
				Roots:                      inclusionRoots,
				Leaves:                     inclusionLeaves,
				InPathIndices:              inclusionInPathIndices,
				InPathElements:             inclusionInPathElements,
				NumberOfCompressedAccounts: inclusionNumberOfCompressedAccounts,
				Depth:                      inclusionTreeDepth,
			},
			NonInclusion: NonInclusionCircuit{
				Roots:                      nonInclusionRoots,
				Values:                     nonInclusionValues,
				LeafLowerRangeValues:       nonInclusionLeafLowerRangeValues,
				LeafHigherRangeValues:      nonInclusionLeafHigherRangeValues,
				NextIndices:                nonInclusionLeafIndices,
				InPathIndices:              nonInclusionInPathIndices,
				InPathElements:             nonInclusionInPathElements,
				NumberOfCompressedAccounts: nonInclusionNumberOfCompressedAccounts,
				Depth:                      nonInclusionTreeDepth,
			},
		}

		// Check if the expected result is "true" or "false"
		expectedResult := splitLine[0]
		if expectedResult == "0" {
			// Run the failing test
			assert.ProverFailed(&circuit, assignment, test.WithBackends(backend.GROTH16), test.WithCurves(ecc.BN254), test.NoSerialization())
		} else if expectedResult == "1" {
			// Run the passing test
			assert.ProverSucceeded(&circuit, assignment, test.WithBackends(backend.GROTH16), test.WithCurves(ecc.BN254), test.NoSerialization())
		} else {
			fmt.Println("Invalid expected result: ", expectedResult)
		}
	}
}
