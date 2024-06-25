package prover

import (
	"fmt"
	"light/light-prover/logging"
	"light/light-prover/prover/poseidon"
	"math/big"
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
	InclusionTreeDepth                     uint32
	InclusionNumberOfCompressedAccounts    uint32
	NonInclusionTreeDepth                  uint32
	NonInclusionNumberOfCompressedAccounts uint32
	ProvingKey                             groth16.ProvingKey
	VerifyingKey                           groth16.VerifyingKey
	ConstraintSystem                       constraint.ConstraintSystem
}

// ProveParentHash gadget generates the ParentHash
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
	Depth                      uint32
}

func (gadget InclusionProof) DefineGadget(api frontend.API) interface{} {
	currentHash := make([]frontend.Variable, gadget.NumberOfCompressedAccounts)
	for proofIndex := 0; proofIndex < int(gadget.NumberOfCompressedAccounts); proofIndex++ {
		hash := MerkleRootGadget{
			Hash:  gadget.Leaves[proofIndex],
			Index: gadget.InPathIndices[proofIndex],
			Path:  gadget.InPathElements[proofIndex],
			Depth: int(gadget.Depth)}
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
	Depth                      uint32
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

		hash := MerkleRootGadget{
			Hash:  currentHash[proofIndex],
			Index: gadget.InPathIndices[proofIndex],
			Path:  gadget.InPathElements[proofIndex],
			Depth: int(gadget.Depth)}
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

type LeafHashGadget struct {
	LeafLowerRangeValue  frontend.Variable
	NextIndex            frontend.Variable
	LeafHigherRangeValue frontend.Variable
	Value                frontend.Variable
}

// Limit the number of bits to 248 + 1,
// since we truncate address values to 31 bytes.
func (gadget LeafHashGadget) DefineGadget(api frontend.API) interface{} {
	api.AssertIsDifferent(gadget.LeafLowerRangeValue, gadget.Value)
	// Lower bound is less than value
	AssertIsLess{A: gadget.LeafLowerRangeValue, B: gadget.Value, N: 248}.DefineGadget(api)
	// Value is less than upper bound
	AssertIsLess{A: gadget.Value, B: gadget.LeafHigherRangeValue, N: 248}.DefineGadget(api)
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
	num := api.Add(gadget.A, api.Sub(oneShifted, gadget.B))
	bin := api.ToBinary(num, gadget.N+1)
	api.AssertIsEqual(0, bin[gadget.N])
	return nil
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
		gadget.Hash = abstractor.Call(api, ProveParentHash{Bit: currentPath[i], Hash: gadget.Hash, Sibling: gadget.Path[i]})
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

func GetKeys(keysDir string, circuitTypes []CircuitType) []string {
	var keys []string

	if IsCircuitEnabled(circuitTypes, Inclusion) {
		keys = append(keys, keysDir+"inclusion_26_1.key")
		keys = append(keys, keysDir+"inclusion_26_2.key")
		keys = append(keys, keysDir+"inclusion_26_3.key")
		keys = append(keys, keysDir+"inclusion_26_4.key")
		keys = append(keys, keysDir+"inclusion_26_8.key")
	}
	if IsCircuitEnabled(circuitTypes, NonInclusion) {
		keys = append(keys, keysDir+"non-inclusion_26_1.key")
		keys = append(keys, keysDir+"non-inclusion_26_2.key")
	}
	if IsCircuitEnabled(circuitTypes, Combined) {
		keys = append(keys, keysDir+"combined_26_1_1.key")
		keys = append(keys, keysDir+"combined_26_1_2.key")
		keys = append(keys, keysDir+"combined_26_2_1.key")
		keys = append(keys, keysDir+"combined_26_2_2.key")
		keys = append(keys, keysDir+"combined_26_3_1.key")
		keys = append(keys, keysDir+"combined_26_3_2.key")
		keys = append(keys, keysDir+"combined_26_4_1.key")
		keys = append(keys, keysDir+"combined_26_4_2.key")
	}
	return keys
}
