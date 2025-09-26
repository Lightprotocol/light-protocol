package v2

import (
	"fmt"
	"light/light-prover/logging"
	"light/light-prover/prover/common"
	"math/big"
	"strconv"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/constraint"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
)

type InclusionInputs struct {
	Root            big.Int
	PathIndex       uint32
	PathElements    []big.Int
	Leaf            big.Int
	PublicInputHash big.Int
}

type InclusionParameters struct {
	PublicInputHash big.Int
	Inputs          []InclusionInputs
}

func (p *InclusionParameters) NumberOfCompressedAccounts() uint32 {
	return uint32(len(p.Inputs))
}

func (p *InclusionParameters) TreeHeight() uint32 {
	if len(p.Inputs) == 0 {
		return 0
	}
	return uint32(len(p.Inputs[0].PathElements))
}

func (p *InclusionParameters) ValidateShape(treeHeight uint32, numOfCompressedAccounts uint32) error {
	if p.NumberOfCompressedAccounts() != numOfCompressedAccounts {
		return fmt.Errorf("wrong number of compressed accounts: %d", p.NumberOfCompressedAccounts())
	}
	if p.TreeHeight() != treeHeight {
		return fmt.Errorf("wrong size of merkle proof for proof %d: %d", p.NumberOfCompressedAccounts(), p.TreeHeight())
	}
	return nil
}

func R1CSInclusion(treeHeight uint32, numberOfCompressedAccounts uint32) (constraint.ConstraintSystem, error) {
	roots := make([]frontend.Variable, numberOfCompressedAccounts)
	leaves := make([]frontend.Variable, numberOfCompressedAccounts)
	inPathIndices := make([]frontend.Variable, numberOfCompressedAccounts)
	inPathElements := make([][]frontend.Variable, numberOfCompressedAccounts)

	for i := 0; i < int(numberOfCompressedAccounts); i++ {
		inPathElements[i] = make([]frontend.Variable, treeHeight)
	}
	publicInputHash := frontend.Variable(0)

	circuit := InclusionCircuit{
		PublicInputHash:            publicInputHash,
		Height:                     treeHeight,
		NumberOfCompressedAccounts: numberOfCompressedAccounts,
		Roots:                      roots,
		Leaves:                     leaves,
		InPathIndices:              inPathIndices,
		InPathElements:             inPathElements,
	}
	return frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &circuit)
}

func SetupInclusion(treeHeight uint32, numberOfCompressedAccounts uint32) (*common.MerkleProofSystem, error) {
	ccs, err := R1CSInclusion(treeHeight, numberOfCompressedAccounts)
	if err != nil {
		return nil, err
	}
	pk, vk, err := groth16.Setup(ccs)
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

func ProveInclusion(ps *common.MerkleProofSystem, params *InclusionParameters) (*common.Proof, error) {
	if err := params.ValidateShape(ps.InclusionTreeHeight, ps.InclusionNumberOfCompressedAccounts); err != nil {
		return nil, err
	}

	inPathIndices := make([]frontend.Variable, ps.InclusionNumberOfCompressedAccounts)
	roots := make([]frontend.Variable, ps.InclusionNumberOfCompressedAccounts)
	leaves := make([]frontend.Variable, ps.InclusionNumberOfCompressedAccounts)
	inPathElements := make([][]frontend.Variable, ps.InclusionNumberOfCompressedAccounts)

	for i := 0; i < int(ps.InclusionNumberOfCompressedAccounts); i++ {
		roots[i] = params.Inputs[i].Root
		leaves[i] = params.Inputs[i].Leaf
		inPathIndices[i] = params.Inputs[i].PathIndex
		inPathElements[i] = make([]frontend.Variable, ps.InclusionTreeHeight)
		for j := 0; j < int(ps.InclusionTreeHeight); j++ {
			inPathElements[i][j] = params.Inputs[i].PathElements[j]
		}
	}

	assignment := InclusionCircuit{
		PublicInputHash: params.PublicInputHash,
		Roots:           roots,
		Leaves:          leaves,
		InPathIndices:   inPathIndices,
		InPathElements:  inPathElements,
	}

	witness, err := frontend.NewWitness(&assignment, ecc.BN254.ScalarField())
	if err != nil {
		return nil, err
	}

	logging.Logger().Info().Msg("Proof inclusion" + strconv.Itoa(int(ps.InclusionTreeHeight)) + " " + strconv.Itoa(int(ps.InclusionNumberOfCompressedAccounts)))
	proof, err := groth16.Prove(ps.ConstraintSystem, ps.ProvingKey, witness)
	if err != nil {
		return nil, err
	}

	return &common.Proof{proof}, nil
}

func VerifyInclusion(ps *common.MerkleProofSystem, publicInputsHash big.Int, proof *common.Proof) error {
	publicAssignment := InclusionCircuit{
		PublicInputHash: publicInputsHash,
	}
	witness, err := frontend.NewWitness(&publicAssignment, ecc.BN254.ScalarField(), frontend.PublicOnly())
	if err != nil {
		return err
	}
	return groth16.Verify(proof.Proof, ps.VerifyingKey, witness)
}
