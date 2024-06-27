package prover

import (
	"light/light-prover/logging"
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

func (p *CombinedParameters) TreeDepth() uint32 {
	return p.InclusionParameters.TreeDepth()
}

func (p *CombinedParameters) NonInclusionNumberOfCompressedAccounts() uint32 {
	return p.NonInclusionParameters.NumberOfCompressedAccounts()
}

func (p *CombinedParameters) NonInclusionTreeDepth() uint32 {
	return p.NonInclusionParameters.TreeDepth()
}

func (p *CombinedParameters) ValidateShape(inclusionTreeDepth uint32, inclusionNumOfCompressedAccounts uint32, nonInclusionTreeDepth uint32, nonInclusionNumOfCompressedAccounts uint32) error {
	if err := p.InclusionParameters.ValidateShape(inclusionTreeDepth, inclusionNumOfCompressedAccounts); err != nil {
		return err
	}
	if err := p.NonInclusionParameters.ValidateShape(nonInclusionTreeDepth, nonInclusionNumOfCompressedAccounts); err != nil {
		return err
	}
	return nil
}

func R1CSCombined(inclusionTreeDepth uint32, inclusionNumberOfCompressedAccounts uint32, nonInclusionTreeDepth uint32, nonInclusionNumberOfCompressedAccounts uint32) (constraint.ConstraintSystem, error) {
	circuit := InitializeCombinedCircuit(inclusionTreeDepth, inclusionNumberOfCompressedAccounts, nonInclusionTreeDepth, nonInclusionNumberOfCompressedAccounts)
	return frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &circuit)
}

func InitializeCombinedCircuit(inclusionTreeDepth uint32, inclusionNumberOfCompressedAccounts uint32, nonInclusionTreeDepth uint32, nonInclusionNumberOfCompressedAccounts uint32) CombinedCircuit {
	inclusionRoots := make([]frontend.Variable, inclusionNumberOfCompressedAccounts)
	inclusionLeaves := make([]frontend.Variable, inclusionNumberOfCompressedAccounts)
	inclusionInPathIndices := make([]frontend.Variable, inclusionNumberOfCompressedAccounts)
	inclusionInPathElements := make([][]frontend.Variable, inclusionNumberOfCompressedAccounts)
	for i := 0; i < int(inclusionNumberOfCompressedAccounts); i++ {
		inclusionInPathElements[i] = make([]frontend.Variable, inclusionTreeDepth)
	}

	nonInclusionRoots := make([]frontend.Variable, nonInclusionNumberOfCompressedAccounts)
	nonInclusionValues := make([]frontend.Variable, nonInclusionNumberOfCompressedAccounts)
	nonInclusionLeafLowerRangeValues := make([]frontend.Variable, nonInclusionNumberOfCompressedAccounts)
	nonInclusionLeafHigherRangeValues := make([]frontend.Variable, nonInclusionNumberOfCompressedAccounts)
	nonInclusionLeafIndices := make([]frontend.Variable, nonInclusionNumberOfCompressedAccounts)

	nonInclusionInPathIndices := make([]frontend.Variable, nonInclusionNumberOfCompressedAccounts)
	nonInclusionInPathElements := make([][]frontend.Variable, nonInclusionNumberOfCompressedAccounts)

	for i := 0; i < int(nonInclusionNumberOfCompressedAccounts); i++ {
		nonInclusionInPathElements[i] = make([]frontend.Variable, nonInclusionTreeDepth)
	}

	circuit := CombinedCircuit{
		Inclusion: InclusionCircuit{
			Roots:                      inclusionRoots,
			Leaves:                     inclusionLeaves,
			InPathIndices:              inclusionInPathIndices,
			InPathElements:             inclusionInPathElements,
			NumberOfCompressedAccounts: inclusionNumberOfCompressedAccounts,
			Depth:                      inclusionTreeDepth,
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
			Depth:                      nonInclusionTreeDepth,
		},
	}
	return circuit
}

func SetupCombined(inclusionTreeDepth uint32, inclusionNumberOfCompressedAccounts uint32, nonInclusionTreeDepth uint32, nonInclusionNumberOfCompressedAccounts uint32) (*ProvingSystem, error) {
	ccs, err := R1CSCombined(inclusionTreeDepth, inclusionNumberOfCompressedAccounts, nonInclusionTreeDepth, nonInclusionNumberOfCompressedAccounts)
	if err != nil {
		return nil, err
	}
	pk, vk, err := groth16.Setup(ccs)
	if err != nil {
		return nil, err
	}
	return &ProvingSystem{inclusionTreeDepth, inclusionNumberOfCompressedAccounts, nonInclusionTreeDepth, nonInclusionNumberOfCompressedAccounts, pk, vk, ccs}, nil
}

func (ps *ProvingSystem) ProveCombined(params *CombinedParameters) (*Proof, error) {
	if err := params.ValidateShape(ps.InclusionTreeDepth, ps.InclusionNumberOfCompressedAccounts, ps.NonInclusionTreeDepth, ps.NonInclusionNumberOfCompressedAccounts); err != nil {
		return nil, err
	}

	circuit := InitializeCombinedCircuit(ps.InclusionTreeDepth, ps.InclusionNumberOfCompressedAccounts, ps.NonInclusionTreeDepth, ps.NonInclusionNumberOfCompressedAccounts)

	for i := 0; i < int(ps.InclusionNumberOfCompressedAccounts); i++ {
		circuit.Inclusion.Roots[i] = params.InclusionParameters.Inputs[i].Root
		circuit.Inclusion.Leaves[i] = params.InclusionParameters.Inputs[i].Leaf
		circuit.Inclusion.InPathIndices[i] = params.InclusionParameters.Inputs[i].PathIndex
		circuit.Inclusion.InPathElements[i] = make([]frontend.Variable, ps.InclusionTreeDepth)
		for j := 0; j < int(ps.InclusionTreeDepth); j++ {
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
		circuit.NonInclusion.InPathElements[i] = make([]frontend.Variable, ps.NonInclusionTreeDepth)
		for j := 0; j < int(ps.NonInclusionTreeDepth); j++ {
			circuit.NonInclusion.InPathElements[i][j] = params.NonInclusionParameters.Inputs[i].PathElements[j]
		}
	}

	witness, err := frontend.NewWitness(&circuit, ecc.BN254.ScalarField())
	if err != nil {
		return nil, err
	}

	logging.Logger().Info().Msg("Proof combined" + strconv.Itoa(int(ps.InclusionTreeDepth)) + " " + strconv.Itoa(int(ps.InclusionNumberOfCompressedAccounts)) + " " + strconv.Itoa(int(ps.NonInclusionTreeDepth)) + " " + strconv.Itoa(int(ps.NonInclusionNumberOfCompressedAccounts)))
	proof, err := groth16.Prove(ps.ConstraintSystem, ps.ProvingKey, witness)
	if err != nil {
		logging.Logger().Error().Msg("combined prove error: " + err.Error())
		return nil, err
	}

	return &Proof{proof}, nil
}
