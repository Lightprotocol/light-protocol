package merkle_tree

import (
	"fmt"
	"math/big"

	"github.com/iden3/go-iden3-crypto/poseidon"
)

type IndexedArray struct {
	Elements         []IndexedElement
	CurrentNodeIndex uint32
}

type IndexedElement struct {
	Value     *big.Int
	NextValue *big.Int
	Index     uint32
}

type IndexedElementBundle struct {
	NewLowElement       IndexedElement
	NewElement          IndexedElement
	NewElementNextValue *big.Int
}

type IndexedMerkleTree struct {
	Tree       *PoseidonTree
	IndexArray *IndexedArray
}

func NewIndexedMerkleTree(height uint32) (*IndexedMerkleTree, error) {
	tree := NewTree(int(height))
	indexArray := &IndexedArray{
		Elements: []IndexedElement{{
			Value:     big.NewInt(0),
			NextValue: big.NewInt(0),
			Index:     0,
		}},
		CurrentNodeIndex: 0,
	}

	return &IndexedMerkleTree{
		Tree:       &tree,
		IndexArray: indexArray,
	}, nil
}

func (ia *IndexedArray) Init() error {
	maxAddr := new(big.Int).Sub(new(big.Int).Lsh(big.NewInt(1), 248), big.NewInt(1))

	ia.Elements = []IndexedElement{{
		Value:     big.NewInt(0),
		NextValue: maxAddr,
		Index:     0,
	}}
	ia.CurrentNodeIndex = 0

	return nil
}

func (ia *IndexedArray) Get(index uint32) *IndexedElement {
	if int(index) >= len(ia.Elements) {
		return nil
	}
	return &ia.Elements[index]
}

func (ia *IndexedArray) Append(value *big.Int) error {
	lowElementIndex, _ := ia.FindLowElementIndex(value)
	lowElement := ia.Elements[lowElementIndex]

	newElementIndex := uint32(len(ia.Elements))
	newElement := IndexedElement{
		Value:     value,
		NextValue: lowElement.NextValue,
		Index:     newElementIndex,
	}

	ia.Elements[lowElementIndex].NextValue = value

	ia.Elements = append(ia.Elements, newElement)
	ia.CurrentNodeIndex = newElementIndex

	return nil
}
func (ia *IndexedArray) FindLowElementIndex(value *big.Int) (uint32, error) {

	for i, element := range ia.Elements {

		// Check if value falls between current and next
		if element.Value.Cmp(value) < 0 && element.NextValue.Cmp(value) > 0 {
			return uint32(i), nil
		}
	}

	return 0, fmt.Errorf("could not find low element index for value %v", value)
}

func (imt *IndexedMerkleTree) Append(value *big.Int) error {
	lowElementIndex, _ := imt.IndexArray.FindLowElementIndex(value)
	lowElement := imt.IndexArray.Get(lowElementIndex)

	if value.Cmp(lowElement.NextValue) >= 0 {
		return fmt.Errorf("new value must be less than next element value")
	}

	newElementIndex := uint32(len(imt.IndexArray.Elements))

	bundle := IndexedElementBundle{
		NewLowElement: IndexedElement{
			Value:     lowElement.Value,
			NextValue: value,
			Index:     lowElement.Index,
		},
		NewElement: IndexedElement{
			Value:     value,
			NextValue: lowElement.NextValue,
			Index:     newElementIndex,
		},
	}

	lowLeafHash, err := HashIndexedElement(&bundle.NewLowElement)
	if err != nil {
		return fmt.Errorf("failed to hash low leaf: %v", err)
	}
	imt.Tree.Update(int(lowElement.Index), *lowLeafHash)

	newLeafHash, err := HashIndexedElement(&bundle.NewElement)
	if err != nil {
		return fmt.Errorf("failed to hash new leaf: %v", err)
	}

	imt.Tree.Update(int(newElementIndex), *newLeafHash)

	imt.IndexArray.Elements[lowElement.Index] = bundle.NewLowElement
	imt.IndexArray.Elements = append(imt.IndexArray.Elements, bundle.NewElement)
	imt.IndexArray.CurrentNodeIndex = newElementIndex

	return nil
}

func (imt *IndexedMerkleTree) Init() error {
	maxAddr := new(big.Int).Sub(new(big.Int).Lsh(big.NewInt(1), 248), big.NewInt(1))

	newLowElement := IndexedElement{
		Value:     big.NewInt(0),
		NextValue: maxAddr,
		Index:     0,
	}

	lowLeafHash, err := HashIndexedElement(&newLowElement)
	if err != nil {
		return fmt.Errorf("failed to hash low leaf: %v", err)
	}
	imt.Tree.Update(0, *lowLeafHash)

	imt.IndexArray.Elements = []IndexedElement{newLowElement}
	imt.IndexArray.CurrentNodeIndex = 1

	return nil
}

func HashIndexedElement(element *IndexedElement) (*big.Int, error) {
	hash, err := poseidon.Hash([]*big.Int{
		element.Value,
		element.NextValue,
	})
	if err != nil {
		return nil, err
	}
	return hash, nil
}

func (imt *IndexedMerkleTree) DeepCopy() *IndexedMerkleTree {
	if imt == nil {
		return nil
	}
	treeCopy := imt.Tree.DeepCopy()

	elementsCopy := make([]IndexedElement, len(imt.IndexArray.Elements))
	for i, element := range imt.IndexArray.Elements {
		elementsCopy[i] = IndexedElement{
			Value:     new(big.Int).Set(element.Value),
			NextValue: new(big.Int).Set(element.NextValue),
			Index:     element.Index,
		}
	}

	indexArrayCopy := &IndexedArray{
		Elements:         elementsCopy,
		CurrentNodeIndex: imt.IndexArray.CurrentNodeIndex,
	}

	return &IndexedMerkleTree{
		Tree:       treeCopy,
		IndexArray: indexArrayCopy,
	}
}

func (imt *IndexedMerkleTree) GetProof(index int) ([]big.Int, error) {
	if index >= len(imt.IndexArray.Elements) {
		return nil, fmt.Errorf("index out of bounds: %d", index)
	}

	proof := imt.Tree.GenerateProof(index)
	return proof, nil
}

func (imt *IndexedMerkleTree) Verify(index int, element *IndexedElement, proof []big.Int) (bool, error) {
	leafHash, err := HashIndexedElement(element)
	if err != nil {
		return false, fmt.Errorf("failed to hash element: %v", err)
	}

	currentHash := leafHash
	depth := len(proof)

	for i := 0; i < depth; i++ {
		var leftVal, rightVal *big.Int

		if indexIsLeft(index, depth-i) {
			leftVal = currentHash
			rightVal = new(big.Int).Set(&proof[i])
		} else {
			leftVal = new(big.Int).Set(&proof[i])
			rightVal = currentHash
		}

		var err error
		currentHash, err = poseidon.Hash([]*big.Int{leftVal, rightVal})
		if err != nil {
			return false, fmt.Errorf("failed to hash proof element: %v", err)
		}
	}

	rootValue := imt.Tree.Root.Value()
	return currentHash.Cmp(&rootValue) == 0, nil
}
