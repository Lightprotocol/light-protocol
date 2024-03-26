package merkle_tree

import (
	"fmt"
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

	// generate trees with depth 1..26 and numberOfUtxos 1..10 and store the serialized results in a file
	for i := 1; i <= 8; i++ {
		for j := 1; j <= 4; j++ {
			tree := BuildTestTree(i, j, true)
			var json, err = tree.MarshalJSON()
			if err != nil {
				t.Errorf("Error marshalling JSON: %v", err)
				return
			}
			_, err = fmt.Fprintf(file, "%d;%s\n", 1, json)
			if err != nil {
				t.Errorf("Error writing to file: %v", err)
				return
			}

			tree = BuildTestTree(i, j, true)
			invalidValue := big.NewInt(999)
			tree.Root[0] = *invalidValue
			json, err = tree.MarshalJSON()
			if err != nil {
				t.Errorf("Error marshalling JSON: %v", err)
				return
			}
			_, err = fmt.Fprintf(file, "%d;%s\n", 0, json)
			if err != nil {
				t.Errorf("Error writing to file: %v", err)
				return
			}

			tree = BuildTestTree(i, j, true)
			tree.Leaf[0] = *invalidValue
			json, err = tree.MarshalJSON()
			if err != nil {
				t.Errorf("Error marshalling JSON: %v", err)
				return
			}
			_, err = fmt.Fprintf(file, "%d;%s\n", 0, json)
			if err != nil {
				t.Errorf("Error writing to file: %v", err)
				return
			}

			tree = BuildTestTree(i, j, true)
			tree.InPathIndices[0] = 999
			json, err = tree.MarshalJSON()
			if err != nil {
				t.Errorf("Error marshalling JSON: %v", err)
				return
			}
			_, err = fmt.Fprintf(file, "%d;%s\n", 0, json)
			if err != nil {
				t.Errorf("Error writing to file: %v", err)
				return
			}

			tree = BuildTestTree(i, j, true)
			tree.InPathElements[0][0] = *invalidValue
			json, err = tree.MarshalJSON()
			if err != nil {
				t.Errorf("Error marshalling JSON: %v", err)
				return
			}
			_, err = fmt.Fprintf(file, "%d;%s\n", 0, json)
			if err != nil {
				t.Errorf("Error writing to file: %v", err)
				return
			}

			if err != nil {
				fmt.Println(err)
				return
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

	// generate trees with depth 1..26 and numberOfUtxos 1..10 and store the serialized results in a file
	for i := 1; i <= 8; i++ {
		for j := 1; j <= 4; j++ {
			tree := BuildTestNonInclusionTree(i, j, true, true)
			var json, err = tree.MarshalJSON()
			if err != nil {
				t.Errorf("Error marshalling JSON: %v", err)
				return
			}
			_, err = fmt.Fprintf(file, "%d;%s\n", 1, json)

			if err != nil {
				t.Errorf("Error writing to file: %v", err)
				return
			}
			tree = BuildTestNonInclusionTree(i, j, true, true)
			invalidValue := big.NewInt(9999)
			tree.Root[0] = *invalidValue
			json, err = tree.MarshalJSON()
			if err != nil {
				t.Errorf("Error marshalling JSON: %v", err)
				return
			}
			_, err = fmt.Fprintf(file, "%d;%s\n", 0, json)

			if err != nil {
				t.Errorf("Error writing to file: %v", err)
				return
			}

			tree = BuildTestNonInclusionTree(i, j, true, false)
			json, err = tree.MarshalJSON()
			if err != nil {
				t.Errorf("Error marshalling JSON: %v", err)
				return
			}
			_, err = fmt.Fprintf(file, "%d;%s\n", 0, json)

			tree = BuildTestNonInclusionTree(i, j, true, true)
			tree.InPathIndices[0] = 9999
			json, err = tree.MarshalJSON()
			if err != nil {
				t.Errorf("Error marshalling JSON: %v", err)
				return
			}
			_, err = fmt.Fprintf(file, "%d;%s\n", 0, json)

			tree = BuildTestNonInclusionTree(i, j, true, true)
			tree.InPathElements[0][0] = *invalidValue
			json, err = tree.MarshalJSON()
			if err != nil {
				t.Errorf("Error marshalling JSON: %v", err)
				return
			}
			_, err = fmt.Fprintf(file, "%d;%s\n", 0, json)
			if err != nil {
				t.Errorf("Error writing to file: %v", err)
				return
			}

		}
	}
}
