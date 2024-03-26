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

		var inclusionNumberOfUtxos = len(params.InclusionParameters.Root)
		var inclusionTreeDepth = len(params.InclusionParameters.InPathElements[0])

		inclusionRoot := make([]frontend.Variable, inclusionNumberOfUtxos)
		for i, v := range params.InclusionParameters.Root {
			inclusionRoot[i] = v
		}

		inclusionLeaf := make([]frontend.Variable, inclusionNumberOfUtxos)
		for i, v := range params.InclusionParameters.Leaf {
			inclusionLeaf[i] = v
		}

		inclusionInPathIndices := make([]frontend.Variable, inclusionNumberOfUtxos)
		for i, v := range params.InclusionParameters.InPathIndices {
			inclusionInPathIndices[i] = v
		}

		inclusionInPathElements := make([][]frontend.Variable, inclusionNumberOfUtxos)
		for i := 0; i < int(inclusionNumberOfUtxos); i++ {
			inclusionInPathElements[i] = make([]frontend.Variable, inclusionTreeDepth)
		}

		for i, v := range params.InclusionParameters.InPathElements {
			for j, v2 := range v {
				inclusionInPathElements[i][j] = v2
			}
		}

		var nonInclusionNumberOfUtxos = len(params.NonInclusionParameters.Root)
		var nonInclusionTreeDepth = len(params.NonInclusionParameters.InPathElements[0])

		root := make([]frontend.Variable, nonInclusionNumberOfUtxos)
		for i, v := range params.NonInclusionParameters.Root {
			root[i] = v
		}

		value := make([]frontend.Variable, nonInclusionNumberOfUtxos)
		for i, v := range params.NonInclusionParameters.Value {
			value[i] = v
		}

		leafLowerRangeValue := make([]frontend.Variable, nonInclusionNumberOfUtxos)
		for i, v := range params.NonInclusionParameters.LeafLowerRangeValue {
			leafLowerRangeValue[i] = v
		}

		leafHigherRangeValue := make([]frontend.Variable, nonInclusionNumberOfUtxos)
		for i, v := range params.NonInclusionParameters.LeafHigherRangeValue {
			leafHigherRangeValue[i] = v
		}

		leafIndex := make([]frontend.Variable, nonInclusionNumberOfUtxos)
		for i, v := range params.NonInclusionParameters.LeafIndex {
			leafIndex[i] = v
		}

		inPathIndices := make([]frontend.Variable, nonInclusionNumberOfUtxos)
		for i, v := range params.NonInclusionParameters.InPathIndices {
			inPathIndices[i] = v
		}

		inPathElements := make([][]frontend.Variable, nonInclusionNumberOfUtxos)
		for i := 0; i < int(nonInclusionNumberOfUtxos); i++ {
			inPathElements[i] = make([]frontend.Variable, nonInclusionTreeDepth)
		}

		for i, v := range params.NonInclusionParameters.InPathElements {
			for j, v2 := range v {
				inPathElements[i][j] = v2
			}
		}

		var circuit CombinedCircuit

		circuit.Inclusion.Root = make([]frontend.Variable, inclusionNumberOfUtxos)
		circuit.Inclusion.Leaf = make([]frontend.Variable, inclusionNumberOfUtxos)
		circuit.Inclusion.InPathIndices = make([]frontend.Variable, inclusionNumberOfUtxos)
		circuit.Inclusion.InPathElements = make([][]frontend.Variable, inclusionNumberOfUtxos)
		for i := 0; i < int(inclusionNumberOfUtxos); i++ {
			circuit.Inclusion.InPathElements[i] = make([]frontend.Variable, inclusionTreeDepth)
		}

		circuit.Inclusion.NumberOfUtxos = int(inclusionNumberOfUtxos)
		circuit.Inclusion.Depth = int(inclusionTreeDepth)

		circuit.NonInclusion.Root = make([]frontend.Variable, nonInclusionNumberOfUtxos)
		circuit.NonInclusion.Value = make([]frontend.Variable, nonInclusionNumberOfUtxos)
		circuit.NonInclusion.LeafLowerRangeValue = make([]frontend.Variable, nonInclusionNumberOfUtxos)
		circuit.NonInclusion.LeafHigherRangeValue = make([]frontend.Variable, nonInclusionNumberOfUtxos)
		circuit.NonInclusion.LeafIndex = make([]frontend.Variable, nonInclusionNumberOfUtxos)
		circuit.NonInclusion.InPathIndices = make([]frontend.Variable, nonInclusionNumberOfUtxos)
		circuit.NonInclusion.InPathElements = make([][]frontend.Variable, nonInclusionNumberOfUtxos)
		for i := 0; i < int(nonInclusionNumberOfUtxos); i++ {
			circuit.NonInclusion.InPathElements[i] = make([]frontend.Variable, nonInclusionTreeDepth)
		}

		circuit.NonInclusion.NumberOfUtxos = int(nonInclusionNumberOfUtxos)
		circuit.NonInclusion.Depth = int(nonInclusionTreeDepth)

		assignment := &CombinedCircuit{
			Inclusion: InclusionCircuit{
				Root:           inclusionRoot,
				Leaf:           inclusionLeaf,
				InPathIndices:  inclusionInPathIndices,
				InPathElements: inclusionInPathElements,
				NumberOfUtxos:  inclusionNumberOfUtxos,
				Depth:          inclusionTreeDepth,
			},
			NonInclusion: NonInclusionCircuit{
				Root:                 root,
				Value:                value,
				LeafLowerRangeValue:  leafLowerRangeValue,
				LeafHigherRangeValue: leafHigherRangeValue,
				LeafIndex:            leafIndex,
				InPathIndices:        inPathIndices,
				InPathElements:       inPathElements,
				NumberOfUtxos:        nonInclusionNumberOfUtxos,
				Depth:                nonInclusionTreeDepth,
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
