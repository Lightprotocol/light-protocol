package merkle_tree

import (
	"fmt"
	"math/big"
	"testing"

	"github.com/consensys/gnark/test"
)

func TestSubtrees(t *testing.T) {
	assert := test.NewAssert(t)
	treeDepth := 4
	tree := NewTree(int(treeDepth))
	subtrees := tree.GetRightmostSubtrees(int(treeDepth))
	fmt.Println("Initial subtrees:")
	for i := 1; i < len(subtrees); i++ {
		subtree := [32](byte)(subtrees[i].Bytes())
		assert.Equal([32](byte)(subtree), EMPTY_TREE[i])
	}

	fmt.Println("Appending 1")
	leaf_0 := new(big.Int).SetInt64(1)
	tree.Update(0, *leaf_0)
	tree.Update(1, *leaf_0)
	subtrees = tree.GetRightmostSubtrees(int(treeDepth))
	fmt.Println("0 Next subtrees:")
	for i := 0; i < len(subtrees); i++ {
		subtree := subtrees[i]
		ref_subtree := new(big.Int).SetBytes(TREE_AFTER_1_UPDATE[i][:])
		assert.Equal(subtree, ref_subtree)
	}

	leaf_1 := new(big.Int).SetInt64(2)
	tree.Update(2, *leaf_1)
	tree.Update(3, *leaf_1)
	subtrees = tree.GetRightmostSubtrees(int(treeDepth))
	fmt.Println("1 Next subtrees:")
	for i := 0; i < len(subtrees); i++ {
		subtree := subtrees[i]
		ref_subtree := new(big.Int).SetBytes(TREE_AFTER_2_UPDATES[i][:])
		assert.Equal(subtree, ref_subtree)
	}

}

var EMPTY_TREE = [4][32]byte{
	{0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0},
	{32, 152, 245, 251, 158, 35, 158, 171, 60, 234, 195, 242, 123, 129, 228, 129, 220, 49, 36, 213, 95, 254, 213, 35, 168, 57, 238, 132, 70, 182, 72, 100},
	{16, 105, 103, 61, 205, 177, 34, 99, 223, 48, 26, 111, 245, 132, 167, 236, 38, 26, 68, 203, 157, 198, 141, 240, 103, 164, 119, 68, 96, 177, 241, 225},
	{24, 244, 51, 49, 83, 126, 226, 175, 46, 61, 117, 141, 80, 247, 33, 6, 70, 124, 110, 234, 80, 55, 29, 213, 40, 213, 126, 178, 184, 86, 210, 56},
}

var TREE_AFTER_1_UPDATE = [4][32]byte{
	{0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1},
	{0, 122, 243, 70, 226, 211, 4, 39, 158, 121, 224, 169, 243, 2, 63, 119, 18, 148, 167, 138, 203, 112, 231, 63, 144, 175, 226, 124, 173, 64, 30, 129},
	{4, 163, 62, 195, 162, 201, 237, 49, 131, 153, 66, 155, 106, 112, 192, 40, 76, 131, 230, 239, 224, 130, 106, 36, 128, 57, 172, 107, 60, 247, 103, 194},
	{7, 118, 172, 114, 242, 52, 137, 62, 111, 106, 113, 139, 123, 161, 39, 255, 86, 13, 105, 167, 223, 52, 15, 29, 137, 37, 106, 178, 49, 44, 226, 75},
}

var TREE_AFTER_2_UPDATES = [4][32]byte{
	{0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2},
	{0, 122, 243, 70, 226, 211, 4, 39, 158, 121, 224, 169, 243, 2, 63, 119, 18, 148, 167, 138, 203, 112, 231, 63, 144, 175, 226, 124, 173, 64, 30, 129},
	{18, 102, 129, 25, 152, 42, 192, 218, 100, 215, 169, 202, 77, 24, 100, 133, 45, 152, 17, 121, 103, 9, 187, 226, 182, 36, 35, 35, 126, 255, 244, 140},
	{11, 230, 92, 56, 65, 91, 231, 137, 40, 92, 11, 193, 90, 225, 123, 79, 82, 17, 212, 147, 43, 41, 126, 223, 49, 2, 139, 211, 249, 138, 7, 12},
}
