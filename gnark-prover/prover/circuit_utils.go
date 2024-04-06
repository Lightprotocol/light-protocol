package prover

import (
	"fmt"
	"light/light-prover/logging"
	"light/light-prover/prover/poseidon"
	"os"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/constraint"
	"github.com/consensys/gnark/frontend"
	"github.com/reilabs/gnark-lean-extractor/v2/abstractor"
)

type Proof struct {
	Proof groth16.Proof
}

type ProvingSystem struct {
	InclusionTreeDepth        uint32
	InclusionNumberOfUtxos    uint32
	NonInclusionTreeDepth     uint32
	NonInclusionNumberOfUtxos uint32
	ProvingKey                groth16.ProvingKey
	VerifyingKey              groth16.VerifyingKey
	ConstraintSystem          constraint.ConstraintSystem
}

// ProofRound gadget generates the ParentHash
type ProofRound struct {
	Direction frontend.Variable
	Hash      frontend.Variable
	Sibling   frontend.Variable
}

func (gadget ProofRound) DefineGadget(api frontend.API) interface{} {
	api.AssertIsBoolean(gadget.Direction)
	d1 := api.Select(gadget.Direction, gadget.Sibling, gadget.Hash)
	d2 := api.Select(gadget.Direction, gadget.Hash, gadget.Sibling)
	sum := abstractor.Call(api, poseidon.Poseidon2{In1: d1, In2: d2})
	return sum
}

type InclusionProof struct {
	Roots          []frontend.Variable
	Leaves         []frontend.Variable
	InPathIndices  []frontend.Variable
	InPathElements [][]frontend.Variable

	NumberOfUtxos int
	Depth         int
}

func (gadget InclusionProof) DefineGadget(api frontend.API) interface{} {
	currentHash := make([]frontend.Variable, gadget.NumberOfUtxos)
	for proofIndex := 0; proofIndex < gadget.NumberOfUtxos; proofIndex++ {
		hash := MerkleRootGadget{
			Hash:  gadget.Leaves[proofIndex],
			Index: gadget.InPathIndices[proofIndex],
			Path:  gadget.InPathElements[proofIndex],
			Depth: gadget.Depth}
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
	LeafIndices           []frontend.Variable

	InPathIndices  []frontend.Variable
	InPathElements [][]frontend.Variable

	NumberOfUtxos int
	Depth         int
}

func (gadget NonInclusionProof) DefineGadget(api frontend.API) interface{} {
	currentHash := make([]frontend.Variable, gadget.NumberOfUtxos)
	for proofIndex := 0; proofIndex < gadget.NumberOfUtxos; proofIndex++ {
		leaf := LeafHashGadget{
			LeafLowerRangeValue:  gadget.LeafLowerRangeValues[proofIndex],
			LeafIndex:            gadget.LeafIndices[proofIndex],
			LeafHigherRangeValue: gadget.LeafHigherRangeValues[proofIndex],
			Value:                gadget.Values[proofIndex]}
		currentHash[proofIndex] = abstractor.Call(api, leaf)

		hash := MerkleRootGadget{
			Hash:  currentHash[proofIndex],
			Index: gadget.InPathIndices[proofIndex],
			Path:  gadget.InPathElements[proofIndex],
			Depth: gadget.Depth}
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
	x := abstractor.Call(api, gadget.InclusionProof)
	y := abstractor.Call(api, gadget.NonInclusionProof)
	if x == nil || y == nil {
		return nil
	}
	return nil
}

type LeafHashGadget struct {
	LeafLowerRangeValue  frontend.Variable
	LeafIndex            frontend.Variable
	LeafHigherRangeValue frontend.Variable
	Value                frontend.Variable
}

func (gadget LeafHashGadget) DefineGadget(api frontend.API) interface{} {
	api.AssertIsDifferent(gadget.LeafLowerRangeValue, gadget.Value)
	api.AssertIsLessOrEqual(gadget.LeafLowerRangeValue, gadget.Value)
	api.AssertIsDifferent(gadget.LeafHigherRangeValue, gadget.Value)
	api.AssertIsLessOrEqual(gadget.Value, gadget.LeafHigherRangeValue)
	return abstractor.Call(api, poseidon.Poseidon3{In1: gadget.LeafLowerRangeValue, In2: gadget.LeafIndex, In3: gadget.LeafHigherRangeValue})
}

type MerkleRootGadget struct {
	Hash  frontend.Variable
	Index frontend.Variable
	Path  []frontend.Variable
	Depth int
}

func (gadget MerkleRootGadget) DefineGadget(api frontend.API) interface{} {
	currentPath := api.ToBinary(gadget.Index, gadget.Depth)
	for i := 0; i < gadget.Depth; i++ {
		gadget.Hash = abstractor.Call(api, ProofRound{Direction: currentPath[i], Hash: gadget.Hash, Sibling: gadget.Path[i]})
	}
	return gadget.Hash
}

// Trusted setup utility functions
// Taken from: https://github.com/bnb-chain/zkbnb/blob/master/common/prove/proof_keys.go#L19
func LoadProvingKey(filepath string) (pk groth16.ProvingKey, err error) {
	logging.Logger().Info().Msg("start reading proving key")
	pk = groth16.NewProvingKey(ecc.BN254)
	f, _ := os.Open(filepath)
	_, err = pk.ReadFrom(f)
	if err != nil {
		return pk, fmt.Errorf("read file error")
	}
	err = f.Close()
	if err != nil {
		return nil, err
	}
	return pk, nil
}

// Taken from: https://github.com/bnb-chain/zkbnb/blob/master/common/prove/proof_keys.go#L32
func LoadVerifyingKey(filepath string) (verifyingKey groth16.VerifyingKey, err error) {
	logging.Logger().Info().Msg("start reading verifying key")
	verifyingKey = groth16.NewVerifyingKey(ecc.BN254)
	f, _ := os.Open(filepath)
	_, err = verifyingKey.ReadFrom(f)
	if err != nil {
		return verifyingKey, fmt.Errorf("read file error")
	}
	err = f.Close()
	if err != nil {
		return nil, err
	}

	return verifyingKey, nil
}
