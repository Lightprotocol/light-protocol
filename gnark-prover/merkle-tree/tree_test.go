package merkle_tree

import (
	"fmt"
	"testing"
)

func TestInclusionParameters_TestTree(t *testing.T) {
	var tree = BuildTestTree(3, 1)
	var json = tree.ToJSON()
	fmt.Println(json)
}
