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

// Iterate over data from csv file "inclusion_test_data.tsv", which contains test data for the inclusion proof.
// The file has two columns, separated by a semicolon.
// First column is the expected result, second column is the input.
// For each row, run the test with the input and check if the result is as expected.
func TestNonInclusion(t *testing.T) {
	assert := test.NewAssert(t)

	file, err := os.Open("../test-data/non_inclusion.csv")
	defer file.Close()

	assert.Nil(err, "Error opening file: ", err)

	scanner := bufio.NewScanner(file)
	for scanner.Scan() {
		line := scanner.Text()
		if line == "" {
			continue
		}
		splitLine := strings.Split(line, ";")
		assert.Equal(len(splitLine), 2, "Invalid line: ", line)

		var params NonInclusionParameters
		err := json.Unmarshal([]byte(splitLine[1]), &params)
		assert.Nil(err, "Error unmarshalling inputs: ", err)

		var numberOfUtxos = len(params.Roots)
		var treeDepth = len(params.InPathElements[0])

		roots := make([]frontend.Variable, numberOfUtxos)
		for i, v := range params.Roots {
			roots[i] = v
		}

		values := make([]frontend.Variable, numberOfUtxos)
		for i, v := range params.Values {
			values[i] = v
		}

		leafLowerRangeValues := make([]frontend.Variable, numberOfUtxos)
		for i, v := range params.LeafLowerRangeValues {
			leafLowerRangeValues[i] = v
		}

		leafHigherRangeValues := make([]frontend.Variable, numberOfUtxos)
		for i, v := range params.LeafHigherRangeValues {
			leafHigherRangeValues[i] = v
		}

		leafIndices := make([]frontend.Variable, numberOfUtxos)
		for i, v := range params.LeafIndices {
			leafIndices[i] = v
		}

		inPathIndices := make([]frontend.Variable, numberOfUtxos)
		for i, v := range params.InPathIndices {
			inPathIndices[i] = v
		}

		inPathElements := make([][]frontend.Variable, numberOfUtxos)
		for i := 0; i < int(numberOfUtxos); i++ {
			inPathElements[i] = make([]frontend.Variable, treeDepth)
		}

		for i, v := range params.InPathElements {
			for j, v2 := range v {
				inPathElements[i][j] = v2
			}
		}

		var circuit NonInclusionCircuit
		circuit.Roots = make([]frontend.Variable, numberOfUtxos)
		circuit.Values = make([]frontend.Variable, numberOfUtxos)
		circuit.LeafLowerRangeValues = make([]frontend.Variable, numberOfUtxos)
		circuit.LeafHigherRangeValues = make([]frontend.Variable, numberOfUtxos)
		circuit.LeafIndices = make([]frontend.Variable, numberOfUtxos)
		circuit.InPathIndices = make([]frontend.Variable, numberOfUtxos)
		circuit.InPathElements = make([][]frontend.Variable, numberOfUtxos)
		for i := 0; i < int(numberOfUtxos); i++ {
			circuit.InPathElements[i] = make([]frontend.Variable, treeDepth)
		}

		circuit.NumberOfUtxos = int(numberOfUtxos)
		circuit.Depth = int(treeDepth)

		// Check if the expected result is "true" or "false"
		expectedResult := splitLine[0]
		if expectedResult == "0" {
			// Run the failing test
			assert.ProverFailed(&circuit, &NonInclusionCircuit{
				Roots:                 roots,
				Values:                values,
				LeafLowerRangeValues:  leafLowerRangeValues,
				LeafHigherRangeValues: leafHigherRangeValues,
				LeafIndices:           leafIndices,
				InPathIndices:         inPathIndices,
				InPathElements:        inPathElements,
				NumberOfUtxos:         numberOfUtxos,
				Depth:                 treeDepth,
			}, test.WithBackends(backend.GROTH16), test.WithCurves(ecc.BN254), test.NoSerialization())
		} else if expectedResult == "1" {
			// Run the passing test
			assert.ProverSucceeded(&circuit, &NonInclusionCircuit{
				Roots:                 roots,
				Values:                values,
				LeafLowerRangeValues:  leafLowerRangeValues,
				LeafHigherRangeValues: leafHigherRangeValues,
				LeafIndices:           leafIndices,
				InPathIndices:         inPathIndices,
				InPathElements:        inPathElements,
				NumberOfUtxos:         numberOfUtxos,
				Depth:                 treeDepth,
			}, test.WithBackends(backend.GROTH16), test.WithCurves(ecc.BN254), test.NoSerialization())
		} else {
			fmt.Println("Invalid expected result: ", expectedResult)
		}
	}
}
