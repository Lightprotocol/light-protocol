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
	root := make([]frontend.Variable, inclusionNumberOfUtxos)
	leaf := make([]frontend.Variable, inclusionNumberOfUtxos)
	inPathIndices := make([]frontend.Variable, inclusionNumberOfUtxos)
	inPathElements := make([][]frontend.Variable, inclusionNumberOfUtxos)
	for i := 0; i < int(inclusionNumberOfUtxos); i++ {
		inPathElements[i] = make([]frontend.Variable, inclusionTreeDepth)
	}

	niRoot := make([]frontend.Variable, nonInclusionNumberOfUtxos)
	niValue := make([]frontend.Variable, nonInclusionNumberOfUtxos)
	niLeafLowerRangeValue := make([]frontend.Variable, nonInclusionNumberOfUtxos)
	niLeafHigherRangeValue := make([]frontend.Variable, nonInclusionNumberOfUtxos)
	niLeafIndex := make([]frontend.Variable, nonInclusionNumberOfUtxos)

	niInPathIndices := make([]frontend.Variable, nonInclusionNumberOfUtxos)
	niInPathElements := make([][]frontend.Variable, nonInclusionNumberOfUtxos)

	for i := 0; i < int(nonInclusionNumberOfUtxos); i++ {
		niInPathElements[i] = make([]frontend.Variable, nonInclusionTreeDepth)
	}

	circuit := CombinedCircuit{
		Inclusion: InclusionCircuit{
			Root:           root,
			Leaf:           leaf,
			InPathIndices:  inPathIndices,
			InPathElements: inPathElements,
			NumberOfUtxos:  int(inclusionNumberOfUtxos),
			Depth:          int(inclusionTreeDepth),
		},
		NonInclusion: NonInclusionCircuit{
			Root:                 niRoot,
			Value:                niValue,
			LeafLowerRangeValue:  niLeafLowerRangeValue,
			LeafHigherRangeValue: niLeafHigherRangeValue,
			LeafIndex:            niLeafIndex,
			InPathIndices:        niInPathIndices,
			InPathElements:       niInPathElements,
			NumberOfUtxos:        int(nonInclusionNumberOfUtxos),
			Depth:                int(nonInclusionTreeDepth),
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
		circuit.Inclusion.Root[i] = params.InclusionParameters.Root[i]
		circuit.Inclusion.Leaf[i] = params.InclusionParameters.Leaf[i]
		circuit.Inclusion.InPathIndices[i] = params.InclusionParameters.InPathIndices[i]
		circuit.Inclusion.InPathElements[i] = make([]frontend.Variable, ps.InclusionTreeDepth)
		for j := 0; j < int(ps.InclusionTreeDepth); j++ {
			circuit.Inclusion.InPathElements[i][j] = params.InclusionParameters.InPathElements[i][j]
		}
	}

	for i := 0; i < int(ps.NonInclusionNumberOfUtxos); i++ {
		circuit.NonInclusion.Root[i] = params.NonInclusionParameters.Root[i]
		circuit.NonInclusion.Value[i] = params.NonInclusionParameters.Value[i]
		circuit.NonInclusion.LeafLowerRangeValue[i] = params.NonInclusionParameters.LeafLowerRangeValue[i]
		circuit.NonInclusion.LeafHigherRangeValue[i] = params.NonInclusionParameters.LeafHigherRangeValue[i]
		circuit.NonInclusion.LeafIndex[i] = params.NonInclusionParameters.LeafIndex[i]
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
