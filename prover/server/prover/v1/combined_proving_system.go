package v1

import (
	"light/light-prover/logging"
	"light/light-prover/prover/common"
	"strconv"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/frontend"
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
	nonInclusionNextIndices := make([]frontend.Variable, nonInclusionNumberOfCompressedAccounts)
	// Initialize NextIndices with 0 to avoid nil values in auto-generated tests
	for i := 0; i < int(nonInclusionNumberOfCompressedAccounts); i++ {
		nonInclusionNextIndices[i] = 0
	}

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
			NextIndices:                nonInclusionNextIndices,
			InPathIndices:              nonInclusionInPathIndices,
			InPathElements:             nonInclusionInPathElements,
			NumberOfCompressedAccounts: nonInclusionNumberOfCompressedAccounts,
			Height:                     nonInclusionTreeHeight,
		},
	}
	return circuit
}

func ProveCombined(ps *common.MerkleProofSystem, params *CombinedParameters) (*common.Proof, error) {
	logging.Logger().Info().Msgf("v1.ProveCombined: Starting with ps.Version=%d, InclusionTreeHeight=%d, InclusionAccounts=%d, NonInclusionTreeHeight=%d, NonInclusionAccounts=%d",
		ps.Version, ps.InclusionTreeHeight, ps.InclusionNumberOfCompressedAccounts,
		ps.NonInclusionTreeHeight, ps.NonInclusionNumberOfCompressedAccounts)

	if err := params.ValidateShape(ps.InclusionTreeHeight, ps.InclusionNumberOfCompressedAccounts, ps.NonInclusionTreeHeight, ps.NonInclusionNumberOfCompressedAccounts); err != nil {
		return nil, err
	}

	circuit := InitializeCombinedCircuit(ps.InclusionTreeHeight, ps.InclusionNumberOfCompressedAccounts, ps.NonInclusionTreeHeight, ps.NonInclusionNumberOfCompressedAccounts)

	for i := 0; i < int(ps.InclusionNumberOfCompressedAccounts); i++ {
		logging.Logger().Debug().Msgf("v1.ProveCombined: Inclusion[%d] Root=%v Leaf=%v PathIndex=%v",
			i, params.InclusionParameters.Inputs[i].Root,
			params.InclusionParameters.Inputs[i].Leaf,
			params.InclusionParameters.Inputs[i].PathIndex)
		circuit.Inclusion.Roots[i] = params.InclusionParameters.Inputs[i].Root
		circuit.Inclusion.Leaves[i] = params.InclusionParameters.Inputs[i].Leaf
		circuit.Inclusion.InPathIndices[i] = params.InclusionParameters.Inputs[i].PathIndex
		circuit.Inclusion.InPathElements[i] = make([]frontend.Variable, ps.InclusionTreeHeight)
		for j := 0; j < int(ps.InclusionTreeHeight); j++ {
			circuit.Inclusion.InPathElements[i][j] = params.InclusionParameters.Inputs[i].PathElements[j]
		}
	}

	for i := 0; i < int(ps.NonInclusionNumberOfCompressedAccounts); i++ {
		logging.Logger().Debug().Msgf("v1.ProveCombined: NonInclusion[%d] Root=%v Value=%v",
			i, params.NonInclusionParameters.Inputs[i].Root,
			params.NonInclusionParameters.Inputs[i].Value)
		logging.Logger().Debug().Msgf("v1.ProveCombined: NonInclusion[%d] LeafLowerRangeValue=%v LeafHigherRangeValue=%v PathIndex=%v",
			i, params.NonInclusionParameters.Inputs[i].LeafLowerRangeValue,
			params.NonInclusionParameters.Inputs[i].LeafHigherRangeValue,
			params.NonInclusionParameters.Inputs[i].PathIndex)

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

	return &common.Proof{proof}, nil
}
