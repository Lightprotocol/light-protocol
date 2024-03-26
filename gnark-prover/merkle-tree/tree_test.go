package merkle_tree

import (
	"fmt"
	"light/light-prover/prover"
	"math/big"
	"os"
	"testing"
)

func TestInclusionParameters_TestTree(t *testing.T) {
	file, err := os.OpenFile("../test-data/inclusion_tmp.csv", os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0644)
	if err != nil {
		fmt.Println(err)
		return
	}
	defer func(file *os.File) {
		err := file.Close()
		if err != nil {
			t.Errorf("Error closing file: %v", err)
		}
	}(file)

	for i := 1; i <= 4; i++ {
		for j := 1; j <= 2; j++ {
			trees := MakeTestIncludedTrees(i, j)
			for _, tree := range trees {
				var json, err = tree.Tree.MarshalJSON()
				if err != nil {
					t.Errorf("Error marshalling JSON: %v", err)
					return
				}

				_, err = fmt.Fprintf(file, "%d;%s\n", flag(tree.Valid), json)
				if err != nil {
					t.Errorf("Error writing to file: %v", err)
					return
				}
			}
		}
	}
}

func TestNonInclusionParameters_TestTree(t *testing.T) {
	file, err := os.OpenFile("../test-data/non-inclusion_tmp.csv", os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0644)
	if err != nil {
		t.Errorf("Error opening file: %v", err)
		return
	}
	defer func(file *os.File) {
		err := file.Close()
		if err != nil {
			t.Errorf("Error closing file: %v", err)
		}
	}(file)

	for i := 1; i <= 4; i++ {
		for j := 1; j <= 2; j++ {
			trees := MakeTestNonInclusionTrees(i, j)
			for _, tree := range trees {
				var json, err = tree.Tree.MarshalJSON()
				if err != nil {
					t.Errorf("Error marshalling JSON: %v", err)
					return
				}
				_, err = fmt.Fprintf(file, "%d;%s\n", flag(tree.Valid), json)
				if err != nil {
					t.Errorf("Error writing to file: %v", err)
					return
				}
			}
		}
	}
}

func TestCombined(t *testing.T) {
	file, err := os.OpenFile("../test-data/combined_tmp.csv", os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0644)
	if err != nil {
		t.Errorf("Error opening file: %v", err)
		return
	}
	defer func(file *os.File) {
		err := file.Close()
		if err != nil {
			t.Errorf("Error closing file: %v", err)
		}
	}(file)

	tree1 := BuildTestTree(26, 1, true)
	tree2 := BuildValidTestNonInclusionTree(26, 1, true)

	var combinedParams = prover.CombinedParameters{
		InclusionParameters:    tree1,
		NonInclusionParameters: tree2,
	}

	var json, err2 = combinedParams.MarshalJSON()
	if err2 != nil {
		t.Errorf("Error marshalling JSON: %v", err)
		return
	}
	fmt.Println(string(json))

	for i := 1; i <= 4; i++ {
		for j := 1; j <= 2; j++ {
			trees1 := MakeTestIncludedTrees(i, j)
			trees2 := MakeTestNonInclusionTrees(i, j)
			for k, tree1 := range trees1 {
				for l, tree2 := range trees2 {
					var combinedParams = prover.CombinedParameters{
						InclusionParameters:    tree1.Tree,
						NonInclusionParameters: tree2.Tree,
					}
					var json, err = combinedParams.MarshalJSON()
					if err != nil {
						t.Errorf("Error marshalling JSON: %v", err)
						return
					}

					valid := tree1.Valid && tree2.Valid
					_, err = fmt.Fprintf(file, "%d;%s\n", flag(valid), json)
					if err != nil {
						t.Errorf("Error writing to file: %v", err)
						return
					}
					fmt.Printf("Test %d: %d, %d\n", i, k, l)
				}
			}
		}
	}
}

func flag(valid bool) int {
	if valid {
		return 1
	}
	return 0
}

type InclusionTreeValidPair struct {
	Tree  prover.InclusionParameters
	Valid bool
}

// Function
//
// `MakeTestIncludedTrees`
//
// ```go
// func MakeTestIncludedTrees(depth int, numberOfUtxos int) []InclusionTreeValidPair
// ```
//
// # Description
//
// The `MakeTestIncludedTrees` function creates an array of InclusionTreeValidPair instances for testing.
// The variation between valid and invalid trees helps simulate real-world scenarios and assists in better
// testing for robustness and error-handling.
//
// Parameters:
//
//   - `depth (int)`: Defines the depth of each included tree.
//   - `numberOfUtxos (int)`: Number of unspent transaction outputs (UTXOs) to include in each tree.
//
// Returns:
// - `[]InclusionTreeValidPair`: An array of `InclusionTreeValidPair` instances, each containing
// an `InclusionParameters` instance and a boolean value indicating whether the tree is valid.
//
// Pairs Explanation:
//
// - `validPair`: A valid tree constructed with input parameters. The Valid field is set to `true`.
// - `invalidRootPair`: A valid tree but the root value is invalidated by setting it to an integer 999. The Valid field is set to `false`.
// - `invalidLeafPair`: A valid tree where a leaf value is invalidated by setting it to an integer 999. The Valid field is set to `false`.
// - `invalidInPathIndicesPair`: A valid tree but the InPathIndices value is invalidated by setting it to an integer 999. The Valid field is set to `false`.
// - `invalidInPathElementsPair`: A valid tree where the InPathElements is invalidated by setting a value to an integer 999. The Valid field is set to `false`.
//
// Example usage:
//
// ```go
// trees := MakeTestIncludedTrees(4, 2)
//
//	for _, tree := range trees {
//	    // perform operations on tree
//	}
//
// ```
func MakeTestIncludedTrees(depth int, numberOfUtxos int) []InclusionTreeValidPair {
	var trees []InclusionTreeValidPair

	validTree := BuildTestTree(depth, numberOfUtxos, true)
	validPair := InclusionTreeValidPair{Tree: validTree, Valid: true}

	invalidRootTree := BuildTestTree(depth, numberOfUtxos, true)
	invalidRootTree.Root[0] = *big.NewInt(999)
	invalidRootPair := InclusionTreeValidPair{Tree: invalidRootTree, Valid: false}

	invalidLeafTree := BuildTestTree(depth, numberOfUtxos, true)
	invalidLeafTree.Leaf[0] = *big.NewInt(999)
	invalidLeafPair := InclusionTreeValidPair{Tree: invalidLeafTree, Valid: false}

	invalidInPathIndicesTree := BuildTestTree(depth, numberOfUtxos, true)
	invalidInPathIndicesTree.InPathIndices[0] = 999
	invalidInPathIndicesPair := InclusionTreeValidPair{Tree: invalidInPathIndicesTree, Valid: false}

	invalidInPathElementsTree := BuildTestTree(depth, numberOfUtxos, true)
	invalidInPathElementsTree.InPathElements[0][0] = *big.NewInt(999)
	invalidInPathElementsPair := InclusionTreeValidPair{Tree: invalidInPathElementsTree, Valid: false}

	trees = append(trees, validPair)
	trees = append(trees, invalidRootPair)
	trees = append(trees, invalidLeafPair)
	trees = append(trees, invalidInPathIndicesPair)
	trees = append(trees, invalidInPathElementsPair)
	return trees
}

type NonInclusionTreeValidPair struct {
	Tree  prover.NonInclusionParameters
	Valid bool
}

// Function
//
// `MakeTestNonInclusionTrees`
//
// ```go
// func MakeTestNonInclusionTrees(depth int, numberOfUtxos int) []NonInclusionTreeValidPair
// ```
//
// # Description
//
// The `MakeTestNonInclusionTrees` function creates an array of `NonInclusionTreeValidPair` instances for testing. These instances include various valid and invalid cases to simulate diverse scenarios and strengthen code robustness and error handling. This function helps in creating a testing environment that closely mimics a variety of real-world scenarios.
//
// # Parameters
//
// - `depth (int)`: Defines the depth of each included tree.
// - `numberOfUtxos (int)`: Number of unspent transaction outputs (UTXOs) to include in each tree.
//
// # Returns
//
// - `[]NonInclusionTreeValidPair`: An array of `NonInclusionTreeValidPair` instances, each containing an `InclusionParameters` instance and a boolean value indicating whether the tree is valid.
//
// # Pairs Explanation
//
// - `validPair`: A tree constructed with input parameters. The `Valid` field is set to `true`.
//
// - `invalidRootPair`: A valid tree but the root value is invalidated by setting it to an integer 999. The `Valid` field is set to `false`.
//
// - `invalidLowValuePair`: An invalid tree with a low value. The `Valid` field is set to `false`.
//
// - `invalidHighValuePair`: An invalid tree with a high value. The `Valid` field is set to `false`.
//
// - `invalidInPathIndicesPair`: A valid tree but the `InPathIndices` value is invalidated by setting it to an integer 999. The `Valid` field is set to `false`.
//
// - `invalidInPathElementsPair`: A valid tree where the `InPathElements` are invalidated by an integer 999. The `Valid` field is set to `false`.
//
// # Example Usage
//
// ```go
// trees := MakeTestNonInclusionTrees(4, 2)
//
//	for _, tree := range trees {
//	    // perform operations on tree
//	}
//
// ```
func MakeTestNonInclusionTrees(depth int, numberOfUtxos int) []NonInclusionTreeValidPair {
	var trees []NonInclusionTreeValidPair

	validTree := BuildValidTestNonInclusionTree(depth, numberOfUtxos, true)
	validPair := NonInclusionTreeValidPair{Tree: validTree, Valid: true}

	invalidRootTree := BuildValidTestNonInclusionTree(depth, numberOfUtxos, true)
	invalidRootTree.Root[0] = *big.NewInt(999)
	invalidRootPair := NonInclusionTreeValidPair{Tree: invalidRootTree, Valid: false}

	invalidLowValueTree := BuildTestNonInclusionTree(depth, numberOfUtxos, true, false, true)
	invalidLowValuePair := NonInclusionTreeValidPair{Tree: invalidLowValueTree, Valid: false}

	invalidHighValueTree := BuildTestNonInclusionTree(depth, numberOfUtxos, true, false, false)
	invalidHighValuePair := NonInclusionTreeValidPair{Tree: invalidHighValueTree, Valid: false}

	invalidInPathIndicesTree := BuildValidTestNonInclusionTree(depth, numberOfUtxos, true)
	invalidInPathIndicesTree.InPathIndices[0] = 999
	invalidInPathIndicesPair := NonInclusionTreeValidPair{Tree: invalidInPathIndicesTree, Valid: false}

	invalidInPathElementsTree := BuildValidTestNonInclusionTree(depth, numberOfUtxos, true)
	invalidInPathElementsTree.InPathElements[0][0] = *big.NewInt(999)
	invalidInPathElementsPair := NonInclusionTreeValidPair{Tree: invalidInPathElementsTree, Valid: false}

	trees = append(trees, validPair)
	trees = append(trees, invalidRootPair)
	trees = append(trees, invalidLowValuePair)
	trees = append(trees, invalidHighValuePair)
	trees = append(trees, invalidInPathIndicesPair)
	trees = append(trees, invalidInPathElementsPair)
	return trees
}
