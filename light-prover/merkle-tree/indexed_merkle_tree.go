package merkle_tree

import (
	"encoding/binary"
	"fmt"
	"math/big"

	"github.com/iden3/go-iden3-crypto/poseidon"
)

type IndexedArray struct {
	Elements         []IndexedElement
	CurrentNodeIndex uint32
	HighestNodeIndex uint32
}

type IndexedElement struct {
	Value     *big.Int
	NextValue *big.Int
	NextIndex uint32
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
			NextIndex: 0,
			Index:     0,
		}},
		CurrentNodeIndex: 0,
		HighestNodeIndex: 0,
	}

	return &IndexedMerkleTree{
		Tree:       &tree,
		IndexArray: indexArray,
	}, nil
}

func (ia *IndexedArray) Init() error {
	maxAddr := new(big.Int).Sub(new(big.Int).Lsh(big.NewInt(1), 248), big.NewInt(1))

	bundle := IndexedElementBundle{
		NewLowElement: IndexedElement{
			Value:     big.NewInt(0),
			NextValue: maxAddr,
			NextIndex: 1,
			Index:     0,
		},
		NewElement: IndexedElement{
			Value:     maxAddr,
			NextValue: big.NewInt(0),
			NextIndex: 0,
			Index:     1,
		},
		NewElementNextValue: big.NewInt(0),
	}

	ia.Elements = []IndexedElement{bundle.NewLowElement, bundle.NewElement}
	ia.CurrentNodeIndex = 1
	ia.HighestNodeIndex = 1

	return nil
}

func (ia *IndexedArray) Get(index uint32) *IndexedElement {
	if int(index) >= len(ia.Elements) {
		return nil
	}
	return &ia.Elements[index]
}

func (ia *IndexedArray) Append(value *big.Int) error {
	lowElementIndex := ia.FindLowElementIndex(value)
	lowElement := ia.Elements[lowElementIndex]

	if lowElement.NextIndex != 0 {
		nextElement := ia.Elements[lowElement.NextIndex]
		if value.Cmp(nextElement.Value) >= 0 {
			return fmt.Errorf("new value must be less than next element value")
		}
	}

	newElementIndex := uint32(len(ia.Elements))
	newElement := IndexedElement{
		Value:     value,
		NextValue: lowElement.NextValue,
		NextIndex: lowElement.NextIndex,
		Index:     newElementIndex,
	}

	ia.Elements[lowElementIndex].NextIndex = newElementIndex
	ia.Elements[lowElementIndex].NextValue = value

	ia.Elements = append(ia.Elements, newElement)
	ia.CurrentNodeIndex = newElementIndex
	if lowElement.NextIndex == 0 {
		ia.HighestNodeIndex = newElementIndex
	}

	return nil
}
func (ia *IndexedArray) FindLowElementIndex(value *big.Int) uint32 {
	maxAddr := new(big.Int).Sub(new(big.Int).Lsh(big.NewInt(1), 248), big.NewInt(1))

	// If we only have initial elements (0 and maxAddr)
	if len(ia.Elements) == 2 {
		// Always return the first element (0) as low element
		return 0
	}

	for i, element := range ia.Elements {
		// Skip the max element
		if element.Value.Cmp(maxAddr) == 0 {
			continue
		}

		// If this is the last element in chain
		if element.NextIndex == 0 {
			return uint32(i)
		}

		nextElement := ia.Get(element.NextIndex)
		if nextElement == nil {
			return uint32(i)
		}

		// Check if value falls between current and next
		if element.Value.Cmp(value) <= 0 && nextElement.Value.Cmp(value) > 0 {
			return uint32(i)
		}
	}

	// If we haven't found a place, return the last non-max element
	for i := len(ia.Elements) - 1; i >= 0; i-- {
		if ia.Elements[i].Value.Cmp(maxAddr) != 0 {
			return uint32(i)
		}
	}

	// Default to first element if nothing else works
	return 0
}

func (imt *IndexedMerkleTree) Append(value *big.Int) error {
	lowElementIndex := imt.IndexArray.FindLowElementIndex(value)
	lowElement := imt.IndexArray.Get(lowElementIndex)

	var nextElement *IndexedElement
	if lowElement.NextIndex != 0 {
		nextElement = imt.IndexArray.Get(lowElement.NextIndex)
		if value.Cmp(nextElement.Value) >= 0 {
			return fmt.Errorf("new value must be less than next element value")
		}
	}

	newElementIndex := uint32(len(imt.IndexArray.Elements))

	bundle := IndexedElementBundle{
		NewLowElement: IndexedElement{
			Value:     lowElement.Value,
			NextValue: value,
			NextIndex: newElementIndex,
			Index:     lowElement.Index,
		},
		NewElement: IndexedElement{
			Value:     value,
			NextValue: nextElement.Value,
			NextIndex: lowElement.NextIndex,
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
	if lowElement.NextIndex == 0 {
		imt.IndexArray.HighestNodeIndex = newElementIndex
	}

	return nil
}

func (imt *IndexedMerkleTree) Init() error {
	maxAddr := new(big.Int).Sub(new(big.Int).Lsh(big.NewInt(1), 248), big.NewInt(1))

	bundle := IndexedElementBundle{
		NewLowElement: IndexedElement{
			Value:     big.NewInt(0),
			NextValue: maxAddr,
			NextIndex: 1,
			Index:     0,
		},
		NewElement: IndexedElement{
			Value:     maxAddr,
			NextValue: big.NewInt(0),
			NextIndex: 0,
			Index:     1,
		},
	}

	lowLeafHash, err := HashIndexedElement(&bundle.NewLowElement)
	if err != nil {
		return fmt.Errorf("failed to hash low leaf: %v", err)
	}
	imt.Tree.Update(0, *lowLeafHash)

	maxLeafHash, err := HashIndexedElement(&bundle.NewElement)
	if err != nil {
		return fmt.Errorf("failed to hash max leaf: %v", err)
	}
	imt.Tree.Update(1, *maxLeafHash)

	imt.IndexArray.Elements = []IndexedElement{bundle.NewLowElement, bundle.NewElement}
	imt.IndexArray.CurrentNodeIndex = 1
	imt.IndexArray.HighestNodeIndex = 1

	return nil
}

func HashIndexedElement(element *IndexedElement) (*big.Int, error) {
	indexBytes := make([]byte, 32)
	binary.BigEndian.PutUint32(indexBytes[28:], element.NextIndex)

	hash, err := poseidon.Hash([]*big.Int{
		element.Value,
		new(big.Int).SetBytes(indexBytes),
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
			NextIndex: element.NextIndex,
			Index:     element.Index,
		}
	}

	indexArrayCopy := &IndexedArray{
		Elements:         elementsCopy,
		CurrentNodeIndex: imt.IndexArray.CurrentNodeIndex,
		HighestNodeIndex: imt.IndexArray.HighestNodeIndex,
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
