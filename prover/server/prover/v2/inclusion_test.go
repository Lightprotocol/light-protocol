package v2

import (
	"bufio"
	"fmt"
	"os"
	"strings"
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/test"
)

// Iterate over data from csv file "inclusion_test_data.tsv", which contains test data for the inclusion proof.
// The file has two columns, separated by a semicolon.
// First column is the expected result, second column is the input.
// For each row, run the test with the input and check if the result is as expected.
func TestInclusion(t *testing.T) {
	assert := test.NewAssert(t)

	file, err := os.Open("../../test-data/inclusion.csv")
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

		var params InclusionParameters
		err := params.UnmarshalJSON([]byte(splitLine[1]))
		assert.Nil(err, "Error unmarshalling inputs: ", err)

		var numberOfCompressedAccounts = params.NumberOfCompressedAccounts()
		var treeHeight = params.TreeHeight()

		roots := make([]frontend.Variable, numberOfCompressedAccounts)
		leaves := make([]frontend.Variable, numberOfCompressedAccounts)
		inPathIndices := make([]frontend.Variable, numberOfCompressedAccounts)
		inPathElements := make([][]frontend.Variable, numberOfCompressedAccounts)
		for i := 0; i < int(numberOfCompressedAccounts); i++ {
			inPathElements[i] = make([]frontend.Variable, treeHeight)
		}

		for i, v := range params.Inputs {
			roots[i] = v.Root
			leaves[i] = v.Leaf
			inPathIndices[i] = v.PathIndex
			for j, v2 := range v.PathElements {
				inPathElements[i][j] = v2
			}

		}

		var circuit InclusionCircuit
		circuit.Roots = make([]frontend.Variable, numberOfCompressedAccounts)
		circuit.Leaves = make([]frontend.Variable, numberOfCompressedAccounts)
		circuit.InPathIndices = make([]frontend.Variable, numberOfCompressedAccounts)
		circuit.InPathElements = make([][]frontend.Variable, numberOfCompressedAccounts)
		for i := 0; i < int(numberOfCompressedAccounts); i++ {
			circuit.InPathElements[i] = make([]frontend.Variable, treeHeight)
		}

		circuit.NumberOfCompressedAccounts = numberOfCompressedAccounts
		circuit.Height = treeHeight

		// Check if the expected result is "true" or "false"
		expectedResult := splitLine[0]
		if expectedResult == "0" {
			// Run the failing test
			assert.ProverFailed(&circuit, &InclusionCircuit{
				PublicInputHash:            params.PublicInputHash,
				Roots:                      roots,
				Leaves:                     leaves,
				InPathIndices:              inPathIndices,
				InPathElements:             inPathElements,
				NumberOfCompressedAccounts: numberOfCompressedAccounts,
				Height:                     treeHeight,
			}, test.WithBackends(backend.GROTH16), test.WithCurves(ecc.BN254), test.NoSerializationChecks())
		} else if expectedResult == "1" {
			// Run the passing test
			assert.ProverSucceeded(&circuit, &InclusionCircuit{
				PublicInputHash:            params.PublicInputHash,
				Roots:                      roots,
				Leaves:                     leaves,
				InPathIndices:              inPathIndices,
				InPathElements:             inPathElements,
				NumberOfCompressedAccounts: numberOfCompressedAccounts,
				Height:                     treeHeight,
			}, test.WithBackends(backend.GROTH16), test.WithCurves(ecc.BN254), test.NoSerializationChecks())
		} else {
			fmt.Println("Invalid expected result: ", expectedResult)
		}
	}
}
