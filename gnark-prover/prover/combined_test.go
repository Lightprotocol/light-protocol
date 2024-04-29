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

		assert.NotEqual(len(params.InclusionParameters.Inputs), 0)
		assert.NotEqual(len(params.NonInclusionParameters.Inputs), 0)

		var inclusionNumberOfUtxos = params.InclusionParameters.NumberOfUTXOs()
		var inclusionTreeDepth = params.InclusionParameters.TreeDepth()

		inclusionRoots := make([]frontend.Variable, inclusionNumberOfUtxos)
		inclusionLeaves := make([]frontend.Variable, inclusionNumberOfUtxos)
		inclusionInPathIndices := make([]frontend.Variable, inclusionNumberOfUtxos)
		inclusionInPathElements := make([][]frontend.Variable, inclusionNumberOfUtxos)
		for i := 0; i < int(inclusionNumberOfUtxos); i++ {
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

		var nonInclusionNumberOfUtxos = params.NonInclusionParameters.NumberOfUTXOs()
		var nonInclusionTreeDepth = params.NonInclusionParameters.TreeDepth()

		nonInclusionRoots := make([]frontend.Variable, nonInclusionNumberOfUtxos)
		nonInclusionValues := make([]frontend.Variable, nonInclusionNumberOfUtxos)
		nonInclusionLeafLowerRangeValues := make([]frontend.Variable, nonInclusionNumberOfUtxos)
		nonInclusionLeafHigherRangeValues := make([]frontend.Variable, nonInclusionNumberOfUtxos)
		nonInclusionLeafIndices := make([]frontend.Variable, nonInclusionNumberOfUtxos)
		nonInclusionInPathIndices := make([]frontend.Variable, nonInclusionNumberOfUtxos)
		nonInclusionInPathElements := make([][]frontend.Variable, nonInclusionNumberOfUtxos)
		for i := 0; i < int(nonInclusionNumberOfUtxos); i++ {
			nonInclusionInPathElements[i] = make([]frontend.Variable, nonInclusionTreeDepth)
		}

		for i, v := range params.NonInclusionParameters.Inputs {
			nonInclusionRoots[i] = v.Root
			nonInclusionValues[i] = v.Value
			nonInclusionLeafLowerRangeValues[i] = v.LeafLowerRangeValue
			nonInclusionLeafHigherRangeValues[i] = v.LeafHigherRangeValue
			nonInclusionLeafIndices[i] = v.LeafIndex
			nonInclusionInPathIndices[i] = v.PathIndex
			for j, v2 := range v.PathElements {
				nonInclusionInPathElements[i][j] = v2
			}
		}

		var circuit CombinedCircuit
		circuit.Inclusion = InclusionCircuit{}
		circuit.NonInclusion = NonInclusionCircuit{}

		circuit.Inclusion.Roots = make([]frontend.Variable, inclusionNumberOfUtxos)
		circuit.Inclusion.Leaves = make([]frontend.Variable, inclusionNumberOfUtxos)
		circuit.Inclusion.InPathIndices = make([]frontend.Variable, inclusionNumberOfUtxos)
		circuit.Inclusion.InPathElements = make([][]frontend.Variable, inclusionNumberOfUtxos)
		for i := 0; i < int(inclusionNumberOfUtxos); i++ {
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
		for i := 0; i < int(nonInclusionNumberOfUtxos); i++ {
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
