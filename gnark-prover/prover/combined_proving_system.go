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

func (p *CombinedParameters) NumberOfUTXOs() uint32 {
	return p.InclusionParameters.NumberOfUTXOs()
}

func (p *CombinedParameters) TreeDepth() uint32 {
	return p.InclusionParameters.TreeDepth()
}

func (p *CombinedParameters) NonInclusionNumberOfUTXOs() uint32 {
	return p.NonInclusionParameters.NumberOfUTXOs()
}

func (p *CombinedParameters) NonInclusionTreeDepth() uint32 {
	return p.NonInclusionParameters.TreeDepth()
}

func (p *CombinedParameters) ValidateShape(inclusionTreeDepth uint32, inclusionNumOfUTXOs uint32, nonInclusionTreeDepth uint32, nonInclusionNumOfUTXOs uint32) error {
	if err := p.InclusionParameters.ValidateShape(inclusionTreeDepth, inclusionNumOfUTXOs); err != nil {
		return err
	}
	if err := p.NonInclusionParameters.ValidateShape(nonInclusionTreeDepth, nonInclusionNumOfUTXOs); err != nil {
		return err
	}
	return nil
}

func R1CSCombined(inclusionTreeDepth uint32, inclusionNumberOfUtxos uint32, nonInclusionTreeDepth uint32, nonInclusionNumberOfUtxos uint32) (constraint.ConstraintSystem, error) {
	circuit := InitializeCombinedCircuit(inclusionTreeDepth, inclusionNumberOfUtxos, nonInclusionTreeDepth, nonInclusionNumberOfUtxos)
	return frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &circuit)
}

func InitializeCombinedCircuit(inclusionTreeDepth uint32, inclusionNumberOfUtxos uint32, nonInclusionTreeDepth uint32, nonInclusionNumberOfUtxos uint32) CombinedCircuit {
	inclusionRoots := make([]frontend.Variable, inclusionNumberOfUtxos)
	inclusionLeaves := make([]frontend.Variable, inclusionNumberOfUtxos)
	inclusionInPathIndices := make([]frontend.Variable, inclusionNumberOfUtxos)
	inclusionInPathElements := make([][]frontend.Variable, inclusionNumberOfUtxos)
	for i := 0; i < int(inclusionNumberOfUtxos); i++ {
		inclusionInPathElements[i] = make([]frontend.Variable, inclusionTreeDepth)
	}

	nonInclusionRoots := make([]frontend.Variable, nonInclusionNumberOfUtxos)
	nonInclusionValues := make([]frontend.Variable, nonInclusionNumberOfUtxos)
	nonInclusionLeafLowerRangeValues := make([]frontend.Variable, nonInclusionNumberOfUtxos)
	nonInclusionLeafHigherRangeValues := make([]frontend.Variable, nonInclusionNumberOfUtxos)
	nonInclusionLeafIndices := make([]frontend.Variable, nonInclusionNumberOfUtxos)

	nonInclusionInPathIndices := make([]frontend.Variable, nonInclusionNumberOfUtxos)
	nonInclusionInPathElements := make([][]frontend.Variable, nonInclusionNumberOfUtxos)

	for i := 0; i < int(nonInclusionNumberOfUtxos); i++ {
		nonInclusionInPathElements[i] = make([]frontend.Variable, nonInclusionTreeDepth)
	}

	circuit := CombinedCircuit{
		Inclusion: InclusionCircuit{
			Roots:          inclusionRoots,
			Leaves:         inclusionLeaves,
			InPathIndices:  inclusionInPathIndices,
			InPathElements: inclusionInPathElements,
			NumberOfUtxos:  int(inclusionNumberOfUtxos),
			Depth:          int(inclusionTreeDepth),
		},
		NonInclusion: NonInclusionCircuit{
			Roots:                 nonInclusionRoots,
			Values:                nonInclusionValues,
			LeafLowerRangeValues:  nonInclusionLeafLowerRangeValues,
			LeafHigherRangeValues: nonInclusionLeafHigherRangeValues,
			LeafIndices:           nonInclusionLeafIndices,
			InPathIndices:         nonInclusionInPathIndices,
			InPathElements:        nonInclusionInPathElements,
			NumberOfUtxos:         int(nonInclusionNumberOfUtxos),
			Depth:                 int(nonInclusionTreeDepth),
		},
	}
	return circuit
}

func SetupCombined(inclusionTreeDepth uint32, inclusionNumberOfUtxos uint32, nonInclusionTreeDepth uint32, nonInclusionNumberOfUtxos uint32) (*ProvingSystem, error) {
	ccs, err := R1CSCombined(inclusionTreeDepth, inclusionNumberOfUtxos, nonInclusionTreeDepth, nonInclusionNumberOfUtxos)
	if err != nil {
		return nil, err
	}
	pk, vk, err := groth16.Setup(ccs)
	if err != nil {
		return nil, err
	}
	return &ProvingSystem{inclusionTreeDepth, inclusionNumberOfUtxos, nonInclusionTreeDepth, nonInclusionNumberOfUtxos, pk, vk, ccs}, nil
}

func (ps *ProvingSystem) ProveCombined(params *CombinedParameters) (*Proof, error) {
	if err := params.ValidateShape(ps.InclusionTreeDepth, ps.InclusionNumberOfUtxos, ps.NonInclusionTreeDepth, ps.NonInclusionNumberOfUtxos); err != nil {
		return nil, err
	}

	circuit := InitializeCombinedCircuit(ps.InclusionTreeDepth, ps.InclusionNumberOfUtxos, ps.NonInclusionTreeDepth, ps.NonInclusionNumberOfUtxos)

	for i := 0; i < int(ps.InclusionNumberOfUtxos); i++ {
		circuit.Inclusion.Roots[i] = params.InclusionParameters.Roots[i]
		circuit.Inclusion.Leaves[i] = params.InclusionParameters.Leaves[i]
		circuit.Inclusion.InPathIndices[i] = params.InclusionParameters.InPathIndices[i]
		circuit.Inclusion.InPathElements[i] = make([]frontend.Variable, ps.InclusionTreeDepth)
		for j := 0; j < int(ps.InclusionTreeDepth); j++ {
			circuit.Inclusion.InPathElements[i][j] = params.InclusionParameters.InPathElements[i][j]
		}
	}

	for i := 0; i < int(ps.NonInclusionNumberOfUtxos); i++ {
		circuit.NonInclusion.Roots[i] = params.NonInclusionParameters.Roots[i]
		circuit.NonInclusion.Values[i] = params.NonInclusionParameters.Values[i]
		circuit.NonInclusion.LeafLowerRangeValues[i] = params.NonInclusionParameters.LeafLowerRangeValues[i]
		circuit.NonInclusion.LeafHigherRangeValues[i] = params.NonInclusionParameters.LeafHigherRangeValues[i]
		circuit.NonInclusion.LeafIndices[i] = params.NonInclusionParameters.LeafIndices[i]
		circuit.NonInclusion.InPathIndices[i] = params.NonInclusionParameters.InPathIndices[i]
		circuit.NonInclusion.InPathElements[i] = make([]frontend.Variable, ps.NonInclusionTreeDepth)
		for j := 0; j < int(ps.NonInclusionTreeDepth); j++ {
			circuit.NonInclusion.InPathElements[i][j] = params.NonInclusionParameters.InPathElements[i][j]
		}
	}

	witness, err := frontend.NewWitness(&circuit, ecc.BN254.ScalarField())
	if err != nil {
		return nil, err
	}

	logging.Logger().Info().Msg("Proof combined" + strconv.Itoa(int(ps.InclusionTreeDepth)) + " " + strconv.Itoa(int(ps.InclusionNumberOfUtxos)) + " " + strconv.Itoa(int(ps.NonInclusionTreeDepth)) + " " + strconv.Itoa(int(ps.NonInclusionNumberOfUtxos)))
	proof, err := groth16.Prove(ps.ConstraintSystem, ps.ProvingKey, witness)
	if err != nil {
		logging.Logger().Error().Msg("combined prove error: " + err.Error())
		return nil, err
	}

	return &Proof{proof}, nil
}
