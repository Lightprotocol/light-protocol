package v2

import (
	"light/light-prover/prover/common"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
	"github.com/reilabs/gnark-lean-extractor/v3/abstractor"
)

type NonInclusionCircuit struct {
	PublicInputHash frontend.Variable `gnark:",public"`

	// hashed public inputs (but passed as private since they're verified via PublicInputHash)
	Roots  []frontend.Variable `gnark:",secret"`
	Values []frontend.Variable `gnark:",secret"`

	// private inputs
	LeafLowerRangeValues  []frontend.Variable `gnark:",secret"`
	LeafHigherRangeValues []frontend.Variable `gnark:",secret"`

	InPathIndices  []frontend.Variable   `gnark:",secret"`
	InPathElements [][]frontend.Variable `gnark:",secret"`

	NumberOfCompressedAccounts uint32
	Height                     uint32
}

func (circuit *NonInclusionCircuit) Define(api frontend.API) error {
	publicInputsHashChain := createTwoInputsHashChain(api, circuit.Roots, circuit.Values)
	api.AssertIsEqual(circuit.PublicInputHash, publicInputsHashChain)

	proof := common.NonInclusionProof{
		Roots:  circuit.Roots,
		Values: circuit.Values,

		LeafLowerRangeValues:  circuit.LeafLowerRangeValues,
		LeafHigherRangeValues: circuit.LeafHigherRangeValues,

		InPathElements: circuit.InPathElements,
		InPathIndices:  circuit.InPathIndices,

		NumberOfCompressedAccounts: circuit.NumberOfCompressedAccounts,
		Height:                     circuit.Height,
	}
	abstractor.CallVoid(api, proof)
	return nil
}

func ImportNonInclusionSetup(treeHeight uint32, numberOfCompressedAccounts uint32, pkPath string, vkPath string) (*common.MerkleProofSystem, error) {
	roots := make([]frontend.Variable, numberOfCompressedAccounts)
	values := make([]frontend.Variable, numberOfCompressedAccounts)

	leafLowerRangeValues := make([]frontend.Variable, numberOfCompressedAccounts)
	leafHigherRangeValues := make([]frontend.Variable, numberOfCompressedAccounts)

	inPathIndices := make([]frontend.Variable, numberOfCompressedAccounts)
	inPathElements := make([][]frontend.Variable, numberOfCompressedAccounts)

	for i := 0; i < int(numberOfCompressedAccounts); i++ {
		inPathElements[i] = make([]frontend.Variable, treeHeight)
	}

	circuit := NonInclusionCircuit{
		Height:                     treeHeight,
		NumberOfCompressedAccounts: numberOfCompressedAccounts,
		Roots:                      roots,
		Values:                     values,
		LeafLowerRangeValues:       leafLowerRangeValues,
		LeafHigherRangeValues:      leafHigherRangeValues,
		InPathIndices:              inPathIndices,
		InPathElements:             inPathElements,
	}

	ccs, err := frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &circuit)
	if err != nil {
		return nil, err
	}

	pk, err := common.LoadProvingKey(pkPath)

	if err != nil {
		return nil, err
	}

	vk, err := common.LoadVerifyingKey(vkPath)

	if err != nil {
		return nil, err
	}

	return &common.MerkleProofSystem{
		NonInclusionTreeHeight:                 treeHeight,
		NonInclusionNumberOfCompressedAccounts: numberOfCompressedAccounts,
		ProvingKey:                             pk,
		VerifyingKey:                           vk,
		ConstraintSystem:                       ccs}, nil
}
