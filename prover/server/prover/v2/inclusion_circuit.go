package v2

import (
	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
	"github.com/reilabs/gnark-lean-extractor/v3/abstractor"
	"light/light-prover/prover/common"
)

type InclusionCircuit struct {
	PublicInputHash frontend.Variable `gnark:",public"`

	// hashed public inputs (but passed as private since they're verified via PublicInputHash)
	Roots  []frontend.Variable `gnark:",secret"`
	Leaves []frontend.Variable `gnark:",secret"`

	// private inputs
	InPathIndices  []frontend.Variable   `gnark:",secret"`
	InPathElements [][]frontend.Variable `gnark:",secret"`

	NumberOfCompressedAccounts uint32
	Height                     uint32
}

func (circuit *InclusionCircuit) Define(api frontend.API) error {
	publicInputsHashChain := createTwoInputsHashChain(api, circuit.Roots, circuit.Leaves)
	api.AssertIsEqual(circuit.PublicInputHash, publicInputsHashChain)

	abstractor.CallVoid(api, common.InclusionProof{
		Roots:          circuit.Roots,
		Leaves:         circuit.Leaves,
		InPathElements: circuit.InPathElements,
		InPathIndices:  circuit.InPathIndices,

		NumberOfCompressedAccounts: circuit.NumberOfCompressedAccounts,
		Height:                     circuit.Height,
	})
	return nil
}

func ImportInclusionSetup(treeHeight uint32, numberOfCompressedAccounts uint32, pkPath string, vkPath string) (*common.MerkleProofSystem, error) {
	roots := make([]frontend.Variable, numberOfCompressedAccounts)
	leaves := make([]frontend.Variable, numberOfCompressedAccounts)
	inPathIndices := make([]frontend.Variable, numberOfCompressedAccounts)
	inPathElements := make([][]frontend.Variable, numberOfCompressedAccounts)

	for i := 0; i < int(numberOfCompressedAccounts); i++ {
		inPathElements[i] = make([]frontend.Variable, treeHeight)
	}
	circuit := InclusionCircuit{
		Height:                     treeHeight,
		NumberOfCompressedAccounts: numberOfCompressedAccounts,
		Roots:                      roots,
		Leaves:                     leaves,
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
		InclusionTreeHeight:                 treeHeight,
		InclusionNumberOfCompressedAccounts: numberOfCompressedAccounts,
		ProvingKey:                          pk,
		VerifyingKey:                        vk,
		ConstraintSystem:                    ccs}, nil
}
