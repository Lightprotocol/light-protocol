package prover

import (
	"light/light-prover/logging"
	"strconv"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/frontend"
	"github.com/reilabs/gnark-lean-extractor/v2/abstractor"
)

type LegacyCombinedParameters struct {
	InclusionParameters    LegacyInclusionParameters
	NonInclusionParameters LegacyNonInclusionParameters
}

func (p *LegacyCombinedParameters) NumberOfCompressedAccounts() uint32 {
	return p.InclusionParameters.NumberOfCompressedAccounts()
}

func (p *LegacyCombinedParameters) TreeHeight() uint32 {
	return p.InclusionParameters.TreeHeight()
}

func (p *LegacyCombinedParameters) NonInclusionNumberOfCompressedAccounts() uint32 {
	return p.NonInclusionParameters.NumberOfCompressedAccounts()
}

func (p *LegacyCombinedParameters) NonInclusionTreeHeight() uint32 {
	return p.NonInclusionParameters.TreeHeight()
}

func (p *LegacyCombinedParameters) ValidateShape(inclusionTreeHeight uint32, inclusionNumOfCompressedAccounts uint32, nonInclusionTreeHeight uint32, nonInclusionNumOfCompressedAccounts uint32) error {
	if err := p.InclusionParameters.ValidateShape(inclusionTreeHeight, inclusionNumOfCompressedAccounts); err != nil {
		return err
	}
	if err := p.NonInclusionParameters.ValidateShape(nonInclusionTreeHeight, nonInclusionNumOfCompressedAccounts); err != nil {
		return err
	}
	return nil
}

func LegacyInitializeCombinedCircuit(inclusionTreeHeight uint32, inclusionNumberOfCompressedAccounts uint32, nonInclusionTreeHeight uint32, nonInclusionNumberOfCompressedAccounts uint32) LegacyCombinedCircuit {
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

	circuit := LegacyCombinedCircuit{
		Inclusion: LegacyInclusionCircuit{
			Roots:                      inclusionRoots,
			Leaves:                     inclusionLeaves,
			InPathIndices:              inclusionInPathIndices,
			InPathElements:             inclusionInPathElements,
			NumberOfCompressedAccounts: inclusionNumberOfCompressedAccounts,
			Height:                     inclusionTreeHeight,
		},
		NonInclusion: LegacyNonInclusionCircuit{
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

// This is not a function circuit just the fronted api
type LegacyCombinedCircuit struct {
	Inclusion    LegacyInclusionCircuit    `gnark:",input"`
	NonInclusion LegacyNonInclusionCircuit `gnark:",input"`
}

// This is not a function circuit just the fronted api
type LegacyInclusionCircuit struct {
	// hashed public inputs
	Roots  []frontend.Variable `gnark:",public"`
	Leaves []frontend.Variable `gnark:",public"`

	// private inputs
	InPathIndices  []frontend.Variable   `gnark:",input"`
	InPathElements [][]frontend.Variable `gnark:",input"`

	NumberOfCompressedAccounts uint32
	Height                     uint32
}

func (circuit *LegacyInclusionCircuit) Define(api frontend.API) error {
	abstractor.CallVoid(api, InclusionProof{
		Roots:          circuit.Roots,
		Leaves:         circuit.Leaves,
		InPathElements: circuit.InPathElements,
		InPathIndices:  circuit.InPathIndices,

		NumberOfCompressedAccounts: circuit.NumberOfCompressedAccounts,
		Height:                     circuit.Height,
	})
	return nil
}

func (circuit *LegacyCombinedCircuit) Define(api frontend.API) error {

	abstractor.CallVoid(api, InclusionProof{
		Roots:          circuit.Inclusion.Roots,
		Leaves:         circuit.Inclusion.Leaves,
		InPathElements: circuit.Inclusion.InPathElements,
		InPathIndices:  circuit.Inclusion.InPathIndices,

		NumberOfCompressedAccounts: circuit.Inclusion.NumberOfCompressedAccounts,
		Height:                     circuit.Inclusion.Height,
	})

	proof := NonInclusionProof{
		Roots:  circuit.NonInclusion.Roots,
		Values: circuit.NonInclusion.Values,

		LeafLowerRangeValues:  circuit.NonInclusion.LeafLowerRangeValues,
		LeafHigherRangeValues: circuit.NonInclusion.LeafHigherRangeValues,

		InPathElements: circuit.NonInclusion.InPathElements,
		InPathIndices:  circuit.NonInclusion.InPathIndices,

		NumberOfCompressedAccounts: circuit.NonInclusion.NumberOfCompressedAccounts,
		Height:                     circuit.NonInclusion.Height,
	}
	abstractor.Call1(api, proof)
	return nil
}

func (ps *ProvingSystemV1) LegacyProveCombined(params *LegacyCombinedParameters) (*Proof, error) {
	if err := params.ValidateShape(ps.InclusionTreeHeight, ps.InclusionNumberOfCompressedAccounts, ps.NonInclusionTreeHeight, ps.NonInclusionNumberOfCompressedAccounts); err != nil {
		return nil, err
	}

	circuit := LegacyInitializeCombinedCircuit(ps.InclusionTreeHeight, ps.InclusionNumberOfCompressedAccounts, ps.NonInclusionTreeHeight, ps.NonInclusionNumberOfCompressedAccounts)

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
