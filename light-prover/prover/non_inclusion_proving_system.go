package prover

import (
	"fmt"
	"light/light-prover/logging"
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
	NextIndex            uint32
}

type NonInclusionParameters struct {
	Inputs []NonInclusionInputs
}

func (p *NonInclusionParameters) NumberOfCompressedAccounts() uint32 {
	return uint32(len(p.Inputs))
}

func (p *NonInclusionParameters) TreeDepth() uint32 {
	if len(p.Inputs) == 0 {
		return 0
	}
	return uint32(len(p.Inputs[0].PathElements))
}

func (p *NonInclusionParameters) ValidateShape(treeDepth uint32, numOfCompressedAccounts uint32) error {
	if p.NumberOfCompressedAccounts() != numOfCompressedAccounts {
		return fmt.Errorf("wrong number of compressed accounts, p.NumberOfCompressedAccounts: %d, numOfCompressedAccounts = %d", p.NumberOfCompressedAccounts(), numOfCompressedAccounts)
	}
	if p.TreeDepth() != treeDepth {
		return fmt.Errorf("wrong size of merkle proof for proof %d: %d", p.NumberOfCompressedAccounts(), p.TreeDepth())
	}
	return nil
}

func R1CSNonInclusion(treeDepth uint32, numberOfCompressedAccounts uint32) (constraint.ConstraintSystem, error) {
	roots := make([]frontend.Variable, numberOfCompressedAccounts)
	values := make([]frontend.Variable, numberOfCompressedAccounts)

	leafLowerRangeValues := make([]frontend.Variable, numberOfCompressedAccounts)
	leafHigherRangeValues := make([]frontend.Variable, numberOfCompressedAccounts)
	nextIndices := make([]frontend.Variable, numberOfCompressedAccounts)

	inPathIndices := make([]frontend.Variable, numberOfCompressedAccounts)
	inPathElements := make([][]frontend.Variable, numberOfCompressedAccounts)

	for i := 0; i < int(numberOfCompressedAccounts); i++ {
		inPathElements[i] = make([]frontend.Variable, treeDepth)
	}

	circuit := NonInclusionCircuit{
		Depth:                      treeDepth,
		NumberOfCompressedAccounts: numberOfCompressedAccounts,
		Roots:                      roots,
		Values:                     values,
		LeafLowerRangeValues:       leafLowerRangeValues,
		LeafHigherRangeValues:      leafHigherRangeValues,
		NextIndices:                nextIndices,
		InPathIndices:              inPathIndices,
		InPathElements:             inPathElements,
	}
	return frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &circuit)
}

func SetupNonInclusion(treeDepth uint32, numberOfCompressedAccounts uint32) (*ProvingSystem, error) {
	ccs, err := R1CSNonInclusion(treeDepth, numberOfCompressedAccounts)
	if err != nil {
		return nil, err
	}
	pk, vk, err := groth16.Setup(ccs)
	if err != nil {
		return nil, err
	}
	return &ProvingSystem{0, 0, treeDepth, numberOfCompressedAccounts, pk, vk, ccs}, nil
}

func (ps *ProvingSystem) ProveNonInclusion(params *NonInclusionParameters) (*Proof, error) {
	if err := params.ValidateShape(ps.NonInclusionTreeDepth, ps.NonInclusionNumberOfCompressedAccounts); err != nil {
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
		roots[i] = params.Inputs[i].Root
		values[i] = params.Inputs[i].Value
		leafLowerRangeValues[i] = params.Inputs[i].LeafLowerRangeValue
		leafHigherRangeValues[i] = params.Inputs[i].LeafHigherRangeValue
		nextIndices[i] = params.Inputs[i].NextIndex
		inPathIndices[i] = params.Inputs[i].PathIndex
		inPathElements[i] = make([]frontend.Variable, ps.NonInclusionTreeDepth)
		for j := 0; j < int(ps.NonInclusionTreeDepth); j++ {
			inPathElements[i][j] = params.Inputs[i].PathElements[j]
		}
	}

	assignment := NonInclusionCircuit{
		Roots:                 roots,
		Values:                values,
		LeafLowerRangeValues:  leafLowerRangeValues,
		LeafHigherRangeValues: leafHigherRangeValues,
		NextIndices:           nextIndices,
		InPathIndices:         inPathIndices,
		InPathElements:        inPathElements,
	}

	witness, err := frontend.NewWitness(&assignment, ecc.BN254.ScalarField())
	if err != nil {
		return nil, err
	}

	logging.Logger().Info().Msg("Proof non-inclusion" + strconv.Itoa(int(ps.NonInclusionTreeDepth)) + " " + strconv.Itoa(int(ps.NonInclusionNumberOfCompressedAccounts)))
	proof, err := groth16.Prove(ps.ConstraintSystem, ps.ProvingKey, witness)
	if err != nil {
		logging.Logger().Error().Msg("non-inclusion prove error: " + err.Error())
		return nil, err
	}

	return &Proof{proof}, nil
}
