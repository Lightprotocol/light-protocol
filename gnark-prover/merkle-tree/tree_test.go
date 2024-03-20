package merkle_tree

import (
	"fmt"
	"math/big"
	"os"
	"testing"
)

func TestInclusionParameters_TestTree(t *testing.T) {
	var tree = BuildTestTree(3, 1, true)
	var json = tree.ToJSON()
	fmt.Println(json)

	file, err := os.OpenFile("../test-data/inclusion2.csv", os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0644)
	if err != nil {
		fmt.Println(err)
		return
	}
	defer file.Close()

	// generate trees with depth 1..26 and numberOfUtxos 1..10 and store the serialized results in a file
	for i := 1; i <= 8; i++ {
		for j := 1; j <= 4; j++ {
			tree := BuildTestTree(i, j, true)
			json := tree.ToJSON()

			_, err = file.WriteString(fmt.Sprintf("%d;%s\n", 1, json))

			if err != nil {
				fmt.Println(err)
				return
			}

			invalidValue := big.NewInt(999)
			tree.Root[0] = *invalidValue
			json = tree.ToJSON()
			_, err = file.WriteString(fmt.Sprintf("%d;%s\n", 0, json))

			tree.Leaf[0] = *invalidValue
			json = tree.ToJSON()
			_, err = file.WriteString(fmt.Sprintf("%d;%s\n", 0, json))

			tree.InPathIndices[0] = 999
			json = tree.ToJSON()
			_, err = file.WriteString(fmt.Sprintf("%d;%s\n", 0, json))

			tree.InPathElements[0][0] = *invalidValue
			json = tree.ToJSON()
			_, err = file.WriteString(fmt.Sprintf("%d;%s\n", 0, json))

			if err != nil {
				fmt.Println(err)
				return
			}

		}
	}
}
