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

type NonInclusionInputs struct {
	Root         big.Int
	Value        big.Int
	PathIndex    uint32
	PathElements []big.Int

	LeafLowerRangeValue  big.Int
	LeafHigherRangeValue big.Int
}

type NonInclusionParameters struct {
	PublicInputHash big.Int
	Inputs          []NonInclusionInputs
}

func (p *NonInclusionParameters) NumberOfCompressedAccounts() uint32 {
	return uint32(len(p.Inputs))
}

func (p *NonInclusionParameters) TreeHeight() uint32 {
	if len(p.Inputs) == 0 {
		return 0
	}
	return uint32(len(p.Inputs[0].PathElements))
}

func (p *NonInclusionParameters) ValidateShape(treeHeight uint32, numOfCompressedAccounts uint32) error {
	if p.NumberOfCompressedAccounts() != numOfCompressedAccounts {
		return fmt.Errorf("wrong number of compressed accounts, p.NumberOfCompressedAccounts: %d, numOfCompressedAccounts = %d", p.NumberOfCompressedAccounts(), numOfCompressedAccounts)
	}
	if p.TreeHeight() != treeHeight {
		return fmt.Errorf("wrong size of merkle proof for proof %d: %d", p.NumberOfCompressedAccounts(), p.TreeHeight())
	}
	return nil
}

func R1CSNonInclusion(treeHeight uint32, numberOfCompressedAccounts uint32) (constraint.ConstraintSystem, error) {
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
		PublicInputHash:            frontend.Variable(0),
		Height:                     treeHeight,
		NumberOfCompressedAccounts: numberOfCompressedAccounts,
		Roots:                      roots,
		Values:                     values,
		LeafLowerRangeValues:       leafLowerRangeValues,
		LeafHigherRangeValues:      leafHigherRangeValues,
		InPathIndices:              inPathIndices,
		InPathElements:             inPathElements,
	}
	return frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &circuit)
}

func SetupNonInclusion(treeHeight uint32, numberOfCompressedAccounts uint32) (*common.MerkleProofSystem, error) {
	ccs, err := R1CSNonInclusion(treeHeight, numberOfCompressedAccounts)
	if err != nil {
		return nil, err
	}
	pk, vk, err := groth16.Setup(ccs)
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

func ProveNonInclusion(ps *common.MerkleProofSystem, params *NonInclusionParameters) (*common.Proof, error) {
	if err := params.ValidateShape(ps.NonInclusionTreeHeight, ps.NonInclusionNumberOfCompressedAccounts); err != nil {
		return nil, err
	}

	roots := make([]frontend.Variable, ps.NonInclusionNumberOfCompressedAccounts)
	values := make([]frontend.Variable, ps.NonInclusionNumberOfCompressedAccounts)

	leafLowerRangeValues := make([]frontend.Variable, ps.NonInclusionNumberOfCompressedAccounts)
	leafHigherRangeValues := make([]frontend.Variable, ps.NonInclusionNumberOfCompressedAccounts)

	inPathElements := make([][]frontend.Variable, ps.NonInclusionNumberOfCompressedAccounts)
	inPathIndices := make([]frontend.Variable, ps.NonInclusionNumberOfCompressedAccounts)

	for i := 0; i < int(ps.NonInclusionNumberOfCompressedAccounts); i++ {
		roots[i] = params.Inputs[i].Root
		values[i] = params.Inputs[i].Value
		leafLowerRangeValues[i] = params.Inputs[i].LeafLowerRangeValue
		leafHigherRangeValues[i] = params.Inputs[i].LeafHigherRangeValue
		inPathIndices[i] = params.Inputs[i].PathIndex
		inPathElements[i] = make([]frontend.Variable, ps.NonInclusionTreeHeight)
		for j := 0; j < int(ps.NonInclusionTreeHeight); j++ {
			inPathElements[i][j] = params.Inputs[i].PathElements[j]
		}
	}

	assignment := NonInclusionCircuit{
		PublicInputHash:       params.PublicInputHash,
		Roots:                 roots,
		Values:                values,
		LeafLowerRangeValues:  leafLowerRangeValues,
		LeafHigherRangeValues: leafHigherRangeValues,
		InPathIndices:         inPathIndices,
		InPathElements:        inPathElements,
	}

	witness, err := frontend.NewWitness(&assignment, ecc.BN254.ScalarField())
	if err != nil {
		return nil, err
	}

	logging.Logger().Info().Msg("Proof non-inclusion" + strconv.Itoa(int(ps.NonInclusionTreeHeight)) + " " + strconv.Itoa(int(ps.NonInclusionNumberOfCompressedAccounts)))
	proof, err := groth16.Prove(ps.ConstraintSystem, ps.ProvingKey, witness)
	if err != nil {
		logging.Logger().Error().Msg("non-inclusion prove error: " + err.Error())
		return nil, err
	}

	return &common.Proof{proof}, nil
}

func VerifyNonInclusion(ps *common.MerkleProofSystem, publicInputHash big.Int, proof *common.Proof) error {
	publicAssignment := NonInclusionCircuit{
		PublicInputHash: publicInputHash,
	}
	witness, err := frontend.NewWitness(&publicAssignment, ecc.BN254.ScalarField(), frontend.PublicOnly())
	if err != nil {
		return err
	}
	return groth16.Verify(proof.Proof, ps.VerifyingKey, witness)
}
