package prover

import (
	"light/light-prover/logging"
	"math/big"
	"strconv"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/constraint"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
)

type CombinedParameters struct {
	InclusionParameters    InclusionParameters
	NonInclusionParameters NonInclusionParameters
}

func (p *CombinedParameters) NumberOfCompressedAccounts() uint32 {
	return p.InclusionParameters.NumberOfCompressedAccounts()
}

func (p *CombinedParameters) TreeHeight() uint32 {
	return p.InclusionParameters.TreeHeight()
}

func (p *CombinedParameters) NonInclusionNumberOfCompressedAccounts() uint32 {
	return p.NonInclusionParameters.NumberOfCompressedAccounts()
}

func (p *CombinedParameters) NonInclusionTreeHeight() uint32 {
	return p.NonInclusionParameters.TreeHeight()
}

func (p *CombinedParameters) ValidateShape(inclusionTreeHeight uint32, inclusionNumOfCompressedAccounts uint32, nonInclusionTreeHeight uint32, nonInclusionNumOfCompressedAccounts uint32) error {
	if err := p.InclusionParameters.ValidateShape(inclusionTreeHeight, inclusionNumOfCompressedAccounts); err != nil {
		return err
	}
	if err := p.NonInclusionParameters.ValidateShape(nonInclusionTreeHeight, nonInclusionNumOfCompressedAccounts); err != nil {
		return err
	}
	return nil
}

func R1CSCombined(inclusionTreeHeight uint32, inclusionNumberOfCompressedAccounts uint32, nonInclusionTreeHeight uint32, nonInclusionNumberOfCompressedAccounts uint32) (constraint.ConstraintSystem, error) {
	circuit := InitializeCombinedCircuit(inclusionTreeHeight, inclusionNumberOfCompressedAccounts, nonInclusionTreeHeight, nonInclusionNumberOfCompressedAccounts)
	return frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &circuit)
}

func InitializeCombinedCircuit(inclusionTreeHeight uint32, inclusionNumberOfCompressedAccounts uint32, nonInclusionTreeHeight uint32, nonInclusionNumberOfCompressedAccounts uint32) CombinedCircuit {
	inclusionRoots := make([]frontend.Variable, inclusionNumberOfCompressedAccounts)
	inclusionLeaves := make([]frontend.Variable, inclusionNumberOfCompressedAccounts)
	inclusionInPathIndices := make([]frontend.Variable, inclusionNumberOfCompressedAccounts)
	inclusionInPathElements := make([][]frontend.Variable, inclusionNumberOfCompressedAccounts)
	for i := 0; i < int(inclusionNumberOfCompressedAccounts); i++ {
		inclusionInPathElements[i] = make([]frontend.Variable, inclusionTreeHeight)
	}

	nonInclusionRoots := make([]frontend.Variable, nonInclusionNumberOfCompressedAccounts)
	nonInclusionValues := make([]frontend.Variable, nonInclusionNumberOfCompressedAccounts)
	nonInclusionLeafLowerRangeValues := make([]frontend.Variable, nonInclusionNumberOfCompressedAccounts)
	nonInclusionLeafHigherRangeValues := make([]frontend.Variable, nonInclusionNumberOfCompressedAccounts)
	nonInclusionLeafIndices := make([]frontend.Variable, nonInclusionNumberOfCompressedAccounts)

	nonInclusionInPathIndices := make([]frontend.Variable, nonInclusionNumberOfCompressedAccounts)
	nonInclusionInPathElements := make([][]frontend.Variable, nonInclusionNumberOfCompressedAccounts)

	for i := 0; i < int(nonInclusionNumberOfCompressedAccounts); i++ {
		nonInclusionInPathElements[i] = make([]frontend.Variable, nonInclusionTreeHeight)
	}

	circuit := CombinedCircuit{
		Inclusion: InclusionCircuit{
			Roots:                      inclusionRoots,
			Leaves:                     inclusionLeaves,
			InPathIndices:              inclusionInPathIndices,
			InPathElements:             inclusionInPathElements,
			NumberOfCompressedAccounts: inclusionNumberOfCompressedAccounts,
			Height:                     inclusionTreeHeight,
		},
		NonInclusion: NonInclusionCircuit{
			Roots:                      nonInclusionRoots,
			Values:                     nonInclusionValues,
			LeafLowerRangeValues:       nonInclusionLeafLowerRangeValues,
			LeafHigherRangeValues:      nonInclusionLeafHigherRangeValues,
			NextIndices:                nonInclusionLeafIndices,
			InPathIndices:              nonInclusionInPathIndices,
			InPathElements:             nonInclusionInPathElements,
			NumberOfCompressedAccounts: nonInclusionNumberOfCompressedAccounts,
			Height:                     nonInclusionTreeHeight,
		},
	}
	return circuit
}

func SetupCombined(inclusionTreeHeight uint32, inclusionNumberOfCompressedAccounts uint32, nonInclusionTreeHeight uint32, nonInclusionNumberOfCompressedAccounts uint32) (*ProvingSystemV1, error) {
	ccs, err := R1CSCombined(inclusionTreeHeight, inclusionNumberOfCompressedAccounts, nonInclusionTreeHeight, nonInclusionNumberOfCompressedAccounts)
	if err != nil {
		return nil, err
	}
	pk, vk, err := groth16.Setup(ccs)
	if err != nil {
		return nil, err
	}
	return &ProvingSystemV1{
		InclusionTreeHeight:                    inclusionTreeHeight,
		InclusionNumberOfCompressedAccounts:    inclusionNumberOfCompressedAccounts,
		NonInclusionTreeHeight:                 nonInclusionTreeHeight,
		NonInclusionNumberOfCompressedAccounts: nonInclusionNumberOfCompressedAccounts,
		ProvingKey:                             pk,
		VerifyingKey:                           vk,
		ConstraintSystem:                       ccs}, nil

}

func (ps *ProvingSystemV1) ProveCombined(params *CombinedParameters) (*Proof, error) {
	if err := params.ValidateShape(ps.InclusionTreeHeight, ps.InclusionNumberOfCompressedAccounts, ps.NonInclusionTreeHeight, ps.NonInclusionNumberOfCompressedAccounts); err != nil {
		return nil, err
	}

	circuit := InitializeCombinedCircuit(ps.InclusionTreeHeight, ps.InclusionNumberOfCompressedAccounts, ps.NonInclusionTreeHeight, ps.NonInclusionNumberOfCompressedAccounts)

	for i := 0; i < int(ps.InclusionNumberOfCompressedAccounts); i++ {
		circuit.Inclusion.Roots[i] = params.InclusionParameters.Inputs[i].Root
		circuit.Inclusion.Leaves[i] = params.InclusionParameters.Inputs[i].Leaf
		circuit.Inclusion.InPathIndices[i] = params.InclusionParameters.Inputs[i].PathIndex
		circuit.Inclusion.InPathElements[i] = make([]frontend.Variable, ps.InclusionTreeHeight)
		for j := 0; j < int(ps.InclusionTreeHeight); j++ {
			circuit.Inclusion.InPathElements[i][j] = params.InclusionParameters.Inputs[i].PathElements[j]
		}
	}

	for i := 0; i < int(ps.NonInclusionNumberOfCompressedAccounts); i++ {
		circuit.NonInclusion.Roots[i] = params.NonInclusionParameters.Inputs[i].Root
		circuit.NonInclusion.Values[i] = params.NonInclusionParameters.Inputs[i].Value
		circuit.NonInclusion.LeafLowerRangeValues[i] = params.NonInclusionParameters.Inputs[i].LeafLowerRangeValue
		circuit.NonInclusion.LeafHigherRangeValues[i] = params.NonInclusionParameters.Inputs[i].LeafHigherRangeValue
		circuit.NonInclusion.NextIndices[i] = params.NonInclusionParameters.Inputs[i].NextIndex
		circuit.NonInclusion.InPathIndices[i] = params.NonInclusionParameters.Inputs[i].PathIndex
		circuit.NonInclusion.InPathElements[i] = make([]frontend.Variable, ps.NonInclusionTreeHeight)
		for j := 0; j < int(ps.NonInclusionTreeHeight); j++ {
			circuit.NonInclusion.InPathElements[i][j] = params.NonInclusionParameters.Inputs[i].PathElements[j]
		}
	}

	witness, err := frontend.NewWitness(&circuit, ecc.BN254.ScalarField())
	if err != nil {
		return nil, err
	}

	logging.Logger().Info().Msg("Proof combined" + strconv.Itoa(int(ps.InclusionTreeHeight)) + " " + strconv.Itoa(int(ps.InclusionNumberOfCompressedAccounts)) + " " + strconv.Itoa(int(ps.NonInclusionTreeHeight)) + " " + strconv.Itoa(int(ps.NonInclusionNumberOfCompressedAccounts)))
	proof, err := groth16.Prove(ps.ConstraintSystem, ps.ProvingKey, witness)
	if err != nil {
		logging.Logger().Error().Msg("combined prove error: " + err.Error())
		return nil, err
	}

	return &Proof{proof}, nil
}

func (ps *ProvingSystemV1) VerifyCombined(root []big.Int, leaf []big.Int, value []big.Int, proof *Proof) error {
	leaves := make([]frontend.Variable, ps.InclusionNumberOfCompressedAccounts)
	for i, v := range leaf {
		leaves[i] = v
	}

	values := make([]frontend.Variable, ps.InclusionNumberOfCompressedAccounts)
	for i, v := range value {
		values[i] = v
	}

	roots := make([]frontend.Variable, ps.InclusionNumberOfCompressedAccounts)
	for i, v := range root {
		roots[i] = v
	}

	publicAssignment := CombinedCircuit{
		InclusionCircuit{
			Roots:  roots,
			Leaves: leaves,
		},
		NonInclusionCircuit{
			Roots:  roots,
			Values: values,
		},
	}
	witness, err := frontend.NewWitness(&publicAssignment, ecc.BN254.ScalarField(), frontend.PublicOnly())
	if err != nil {
		return err
	}
	return groth16.Verify(proof.Proof, ps.VerifyingKey, witness)
}
