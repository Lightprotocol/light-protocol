package v1

import (
	"fmt"
	"light/light-prover/logging"
	"light/light-prover/prover/common"
	"math/big"
	"strconv"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/frontend"
)

type NonInclusionInputs struct {
	Root         big.Int
	Value        big.Int
	PathIndex    uint32
	PathElements []big.Int

	LeafLowerRangeValue  big.Int
	LeafHigherRangeValue big.Int
	NextIndex            uint32
}

type NonInclusionParameters struct {
	Inputs []NonInclusionInputs
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

func ProveNonInclusion(ps *common.MerkleProofSystem, params *NonInclusionParameters) (*common.Proof, error) {
	if err := params.ValidateShape(ps.NonInclusionTreeHeight, ps.NonInclusionNumberOfCompressedAccounts); err != nil {
		return nil, err
	}

	roots := make([]frontend.Variable, ps.NonInclusionNumberOfCompressedAccounts)
	values := make([]frontend.Variable, ps.NonInclusionNumberOfCompressedAccounts)

	leafLowerRangeValues := make([]frontend.Variable, ps.NonInclusionNumberOfCompressedAccounts)
	leafHigherRangeValues := make([]frontend.Variable, ps.NonInclusionNumberOfCompressedAccounts)
	nextIndices := make([]frontend.Variable, ps.NonInclusionNumberOfCompressedAccounts)

	inPathElements := make([][]frontend.Variable, ps.NonInclusionNumberOfCompressedAccounts)
	inPathIndices := make([]frontend.Variable, ps.NonInclusionNumberOfCompressedAccounts)

	for i := 0; i < int(ps.NonInclusionNumberOfCompressedAccounts); i++ {
		logging.Logger().Debug().Msgf("ProveNonInclusion: Input[%d] NextIndex=%d", i, params.Inputs[i].NextIndex)
		roots[i] = params.Inputs[i].Root
		values[i] = params.Inputs[i].Value
		leafLowerRangeValues[i] = params.Inputs[i].LeafLowerRangeValue
		leafHigherRangeValues[i] = params.Inputs[i].LeafHigherRangeValue
		nextIndices[i] = params.Inputs[i].NextIndex
		inPathIndices[i] = params.Inputs[i].PathIndex
		inPathElements[i] = make([]frontend.Variable, ps.NonInclusionTreeHeight)
		for j := 0; j < int(ps.NonInclusionTreeHeight); j++ {
			inPathElements[i][j] = params.Inputs[i].PathElements[j]
		}
	}

	assignment := NonInclusionCircuit{
		Roots:                      roots,
		Values:                     values,
		LeafLowerRangeValues:       leafLowerRangeValues,
		LeafHigherRangeValues:      leafHigherRangeValues,
		NextIndices:                nextIndices,
		InPathIndices:              inPathIndices,
		InPathElements:             inPathElements,
		NumberOfCompressedAccounts: ps.NonInclusionNumberOfCompressedAccounts,
		Height:                     ps.NonInclusionTreeHeight,
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

	return &common.Proof{Proof: proof}, nil
}
