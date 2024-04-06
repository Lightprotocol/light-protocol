package prover

import (
	"bufio"
	"encoding/json"
	"fmt"
	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/test"
	"os"
	"strings"
	"testing"
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

		var inclusionNumberOfUtxos = len(params.InclusionParameters.Roots)
		var inclusionTreeDepth = len(params.InclusionParameters.InPathElements[0])

		inclusionRoots := make([]frontend.Variable, inclusionNumberOfUtxos)
		for i, v := range params.InclusionParameters.Roots {
			inclusionRoots[i] = v
		}

		inclusionLeaves := make([]frontend.Variable, inclusionNumberOfUtxos)
		for i, v := range params.InclusionParameters.Leaves {
			inclusionLeaves[i] = v
		}

		inclusionInPathIndices := make([]frontend.Variable, inclusionNumberOfUtxos)
		for i, v := range params.InclusionParameters.InPathIndices {
			inclusionInPathIndices[i] = v
		}

		inclusionInPathElements := make([][]frontend.Variable, inclusionNumberOfUtxos)
		for i := 0; i < inclusionNumberOfUtxos; i++ {
			inclusionInPathElements[i] = make([]frontend.Variable, inclusionTreeDepth)
		}

		for i, v := range params.InclusionParameters.InPathElements {
			for j, v2 := range v {
				inclusionInPathElements[i][j] = v2
			}
		}

		var nonInclusionNumberOfUtxos = len(params.NonInclusionParameters.Roots)
		var nonInclusionTreeDepth = len(params.NonInclusionParameters.InPathElements[0])

		nonInclusionRoots := make([]frontend.Variable, nonInclusionNumberOfUtxos)
		for i, v := range params.NonInclusionParameters.Roots {
			nonInclusionRoots[i] = v
		}

		nonInclusionValues := make([]frontend.Variable, nonInclusionNumberOfUtxos)
		for i, v := range params.NonInclusionParameters.Values {
			nonInclusionValues[i] = v
		}

		nonInclusionLeafLowerRangeValues := make([]frontend.Variable, nonInclusionNumberOfUtxos)
		for i, v := range params.NonInclusionParameters.LeafLowerRangeValues {
			nonInclusionLeafLowerRangeValues[i] = v
		}

		nonInclusionLeafHigherRangeValues := make([]frontend.Variable, nonInclusionNumberOfUtxos)
		for i, v := range params.NonInclusionParameters.LeafHigherRangeValues {
			nonInclusionLeafHigherRangeValues[i] = v
		}

		nonInclusionLeafIndices := make([]frontend.Variable, nonInclusionNumberOfUtxos)
		for i, v := range params.NonInclusionParameters.LeafIndices {
			nonInclusionLeafIndices[i] = v
		}

		nonInclusionInPathIndices := make([]frontend.Variable, nonInclusionNumberOfUtxos)
		for i, v := range params.NonInclusionParameters.InPathIndices {
			nonInclusionInPathIndices[i] = v
		}

		nonInclusionInPathElements := make([][]frontend.Variable, nonInclusionNumberOfUtxos)
		for i := 0; i < nonInclusionNumberOfUtxos; i++ {
			nonInclusionInPathElements[i] = make([]frontend.Variable, nonInclusionTreeDepth)
		}

		for i, v := range params.NonInclusionParameters.InPathElements {
			for j, v2 := range v {
				nonInclusionInPathElements[i][j] = v2
			}
		}

		var circuit CombinedCircuit

		circuit.Inclusion.Roots = make([]frontend.Variable, inclusionNumberOfUtxos)
		circuit.Inclusion.Leaves = make([]frontend.Variable, inclusionNumberOfUtxos)
		circuit.Inclusion.InPathIndices = make([]frontend.Variable, inclusionNumberOfUtxos)
		circuit.Inclusion.InPathElements = make([][]frontend.Variable, inclusionNumberOfUtxos)
		for i := 0; i < inclusionNumberOfUtxos; i++ {
			circuit.Inclusion.InPathElements[i] = make([]frontend.Variable, inclusionTreeDepth)
		}

		circuit.Inclusion.NumberOfUtxos = inclusionNumberOfUtxos
		circuit.Inclusion.Depth = inclusionTreeDepth

		circuit.NonInclusion.Roots = make([]frontend.Variable, nonInclusionNumberOfUtxos)
		circuit.NonInclusion.Values = make([]frontend.Variable, nonInclusionNumberOfUtxos)
		circuit.NonInclusion.LeafLowerRangeValues = make([]frontend.Variable, nonInclusionNumberOfUtxos)
		circuit.NonInclusion.LeafHigherRangeValues = make([]frontend.Variable, nonInclusionNumberOfUtxos)
		circuit.NonInclusion.LeafIndices = make([]frontend.Variable, nonInclusionNumberOfUtxos)
		circuit.NonInclusion.InPathIndices = make([]frontend.Variable, nonInclusionNumberOfUtxos)
		circuit.NonInclusion.InPathElements = make([][]frontend.Variable, nonInclusionNumberOfUtxos)
		for i := 0; i < nonInclusionNumberOfUtxos; i++ {
			circuit.NonInclusion.InPathElements[i] = make([]frontend.Variable, nonInclusionTreeDepth)
		}

		circuit.NonInclusion.NumberOfUtxos = nonInclusionNumberOfUtxos
		circuit.NonInclusion.Depth = nonInclusionTreeDepth

		assignment := &CombinedCircuit{
			Inclusion: InclusionCircuit{
				Roots:          inclusionRoots,
				Leaves:         inclusionLeaves,
				InPathIndices:  inclusionInPathIndices,
				InPathElements: inclusionInPathElements,
				NumberOfUtxos:  inclusionNumberOfUtxos,
				Depth:          inclusionTreeDepth,
			},
			NonInclusion: NonInclusionCircuit{
				Roots:                 nonInclusionRoots,
				Values:                nonInclusionValues,
				LeafLowerRangeValues:  nonInclusionLeafLowerRangeValues,
				LeafHigherRangeValues: nonInclusionLeafHigherRangeValues,
				LeafIndices:           nonInclusionLeafIndices,
				InPathIndices:         nonInclusionInPathIndices,
				InPathElements:        nonInclusionInPathElements,
				NumberOfUtxos:         nonInclusionNumberOfUtxos,
				Depth:                 nonInclusionTreeDepth,
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
