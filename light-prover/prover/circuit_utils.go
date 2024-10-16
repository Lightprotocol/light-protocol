package prover

import (
	"light/light-prover/prover/poseidon"
	"math/big"

	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/constraint"
	"github.com/consensys/gnark/frontend"

	"github.com/reilabs/gnark-lean-extractor/v2/abstractor"
)

type Proof struct {
	Proof groth16.Proof
}

type ProvingSystemV1 struct {
	InclusionTreeHeight                    uint32
	InclusionNumberOfCompressedAccounts    uint32
	NonInclusionTreeHeight                 uint32
	NonInclusionNumberOfCompressedAccounts uint32
	ProvingKey                             groth16.ProvingKey
	VerifyingKey                           groth16.VerifyingKey
	ConstraintSystem                       constraint.ConstraintSystem
}

type ProvingSystemV2 struct {
	CircuitType      CircuitType
	TreeHeight       uint32
	BatchSize        uint32
	ProvingKey       groth16.ProvingKey
	VerifyingKey     groth16.VerifyingKey
	ConstraintSystem constraint.ConstraintSystem
}

type ProveParentHash struct {
	Bit     frontend.Variable
	Hash    frontend.Variable
	Sibling frontend.Variable
}

func (gadget ProveParentHash) DefineGadget(api frontend.API) interface{} {
	api.AssertIsBoolean(gadget.Bit)
	d1 := api.Select(gadget.Bit, gadget.Sibling, gadget.Hash)
	d2 := api.Select(gadget.Bit, gadget.Hash, gadget.Sibling)
	hash := abstractor.Call(api, poseidon.Poseidon2{In1: d1, In2: d2})
	return hash
}

type InclusionProof struct {
	Roots          []frontend.Variable
	Leaves         []frontend.Variable
	InPathIndices  []frontend.Variable
	InPathElements [][]frontend.Variable

	NumberOfCompressedAccounts uint32
	Height                     uint32
}

func (gadget InclusionProof) DefineGadget(api frontend.API) interface{} {
	currentHash := make([]frontend.Variable, gadget.NumberOfCompressedAccounts)
	for proofIndex := 0; proofIndex < int(gadget.NumberOfCompressedAccounts); proofIndex++ {
		currentPath := api.ToBinary(gadget.InPathIndices[proofIndex], int(gadget.Height))
		hash := MerkleRootGadget{
			Hash:   gadget.Leaves[proofIndex],
			Index:  currentPath,
			Path:   gadget.InPathElements[proofIndex],
			Height: int(gadget.Height)}
		currentHash[proofIndex] = abstractor.Call(api, hash)
		api.AssertIsEqual(currentHash[proofIndex], gadget.Roots[proofIndex])
	}
	return currentHash
}

type NonInclusionProof struct {
	Roots  []frontend.Variable
	Values []frontend.Variable

	LeafLowerRangeValues  []frontend.Variable
	LeafHigherRangeValues []frontend.Variable
	NextIndices           []frontend.Variable

	InPathIndices  []frontend.Variable
	InPathElements [][]frontend.Variable

	NumberOfCompressedAccounts uint32
	Height                     uint32
}

func (gadget NonInclusionProof) DefineGadget(api frontend.API) interface{} {
	currentHash := make([]frontend.Variable, gadget.NumberOfCompressedAccounts)
	for proofIndex := 0; proofIndex < int(gadget.NumberOfCompressedAccounts); proofIndex++ {
		leaf := LeafHashGadget{
			LeafLowerRangeValue:  gadget.LeafLowerRangeValues[proofIndex],
			NextIndex:            gadget.NextIndices[proofIndex],
			LeafHigherRangeValue: gadget.LeafHigherRangeValues[proofIndex],
			Value:                gadget.Values[proofIndex]}
		currentHash[proofIndex] = abstractor.Call(api, leaf)

		currentPath := api.ToBinary(gadget.InPathIndices[proofIndex], int(gadget.Height))
		hash := MerkleRootGadget{
			Hash:   currentHash[proofIndex],
			Index:  currentPath,
			Path:   gadget.InPathElements[proofIndex],
			Height: int(gadget.Height)}
		currentHash[proofIndex] = abstractor.Call(api, hash)
		api.AssertIsEqual(currentHash[proofIndex], gadget.Roots[proofIndex])
	}
	return currentHash
}

type CombinedProof struct {
	InclusionProof    InclusionProof
	NonInclusionProof NonInclusionProof
}

func (gadget CombinedProof) DefineGadget(api frontend.API) interface{} {
	abstractor.Call(api, gadget.InclusionProof)
	abstractor.Call(api, gadget.NonInclusionProof)
	return nil
}

type VerifyProof struct {
	Leaf  frontend.Variable
	Path  []frontend.Variable
	Proof []frontend.Variable
}

func (gadget VerifyProof) DefineGadget(api frontend.API) interface{} {
	currentHash := gadget.Leaf
	for i := 0; i < len(gadget.Path); i++ {
		currentHash = abstractor.Call(api, ProveParentHash{
			Bit:     gadget.Path[i],
			Hash:    currentHash,
			Sibling: gadget.Proof[i],
		})
	}
	return currentHash
}

type LeafHashGadget struct {
	LeafLowerRangeValue  frontend.Variable
	NextIndex            frontend.Variable
	LeafHigherRangeValue frontend.Variable
	Value                frontend.Variable
}

// Limit the number of bits to 248 + 1,
// since we truncate address values to 31 bytes.
func (gadget LeafHashGadget) DefineGadget(api frontend.API) interface{} {
	// Lower bound is less than value
	abstractor.CallVoid(api, AssertIsLess{A: gadget.LeafLowerRangeValue, B: gadget.Value, N: 248})
	// Value is less than upper bound
	abstractor.CallVoid(api, AssertIsLess{A: gadget.Value, B: gadget.LeafHigherRangeValue, N: 248})
	return abstractor.Call(api, poseidon.Poseidon3{In1: gadget.LeafLowerRangeValue, In2: gadget.NextIndex, In3: gadget.LeafHigherRangeValue})
}

// Assert A is less than B.
type AssertIsLess struct {
	A frontend.Variable
	B frontend.Variable
	N int
}

// To prevent overflows N (the number of bits) must not be greater than 252 + 1,
// see https://github.com/zkopru-network/zkopru/issues/116
func (gadget AssertIsLess) DefineGadget(api frontend.API) interface{} {
	// Add 2^N to B to ensure a positive number
	oneShifted := new(big.Int).Lsh(big.NewInt(1), uint(gadget.N))
	num := api.Add(gadget.A, api.Sub(*oneShifted, gadget.B))
	api.ToBinary(num, gadget.N)
	return []frontend.Variable{}
}

type MerkleRootGadget struct {
	Hash   frontend.Variable
	Index  []frontend.Variable
	Path   []frontend.Variable
	Height int
}

func (gadget MerkleRootGadget) DefineGadget(api frontend.API) interface{} {
	for i := 0; i < gadget.Height; i++ {
		gadget.Hash = abstractor.Call(api, ProveParentHash{Bit: gadget.Index[i], Hash: gadget.Hash, Sibling: gadget.Path[i]})
	}
	return gadget.Hash
}

type MerkleRootUpdateGadget struct {
	OldRoot     frontend.Variable
	OldLeaf     frontend.Variable
	NewLeaf     frontend.Variable
	PathIndex   []frontend.Variable
	MerkleProof []frontend.Variable
	Height      int
}

func (gadget MerkleRootUpdateGadget) DefineGadget(api frontend.API) interface{} {
	// Verify the old root
	currentRoot := abstractor.Call(api, MerkleRootGadget{
		Hash:   gadget.OldLeaf,
		Index:  gadget.PathIndex,
		Path:   gadget.MerkleProof,
		Height: gadget.Height,
	})
	api.AssertIsEqual(currentRoot, gadget.OldRoot)

	// Calculate the new root
	newRoot := abstractor.Call(api, MerkleRootGadget{
		Hash:   gadget.NewLeaf,
		Index:  gadget.PathIndex,
		Path:   gadget.MerkleProof,
		Height: gadget.Height,
	})

	return newRoot
}
