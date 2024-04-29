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

type NonInclusionParameters struct {
	Roots  []big.Int
	Values []big.Int

	LeafLowerRangeValues  []big.Int
	LeafHigherRangeValues []big.Int
	LeafIndices           []uint32

	InPathIndices  []uint32
	InPathElements [][]big.Int
}

func (p *NonInclusionParameters) NumberOfUTXOs() uint32 {
	return uint32(len(p.Roots))
}

func (p *NonInclusionParameters) TreeDepth() uint32 {
	if len(p.InPathElements) == 0 {
		return 0
	}
	return uint32(len(p.InPathElements[0]))
}

func (p *NonInclusionParameters) ValidateShape(treeDepth uint32, numOfUTXOs uint32) error {
	if p.NumberOfUTXOs() != numOfUTXOs {
		return fmt.Errorf("wrong number of utxos: %d", len(p.Roots))
	}
	if p.TreeDepth() != treeDepth {
		return fmt.Errorf("wrong size of merkle proof for proof %d: %d", p.NumberOfUTXOs(), p.TreeDepth())
	}
	return nil
}

func R1CSNonInclusion(treeDepth uint32, numberOfUtxos uint32) (constraint.ConstraintSystem, error) {
	roots := make([]frontend.Variable, numberOfUtxos)
	values := make([]frontend.Variable, numberOfUtxos)

	leafLowerRangeValues := make([]frontend.Variable, numberOfUtxos)
	leafHigherRangeValues := make([]frontend.Variable, numberOfUtxos)
	leafIndices := make([]frontend.Variable, numberOfUtxos)

	inPathIndices := make([]frontend.Variable, numberOfUtxos)
	inPathElements := make([][]frontend.Variable, numberOfUtxos)

	for i := 0; i < int(numberOfUtxos); i++ {
		inPathElements[i] = make([]frontend.Variable, treeDepth)
	}

	circuit := NonInclusionCircuit{
		Depth:                 int(treeDepth),
		NumberOfUtxos:         int(numberOfUtxos),
		Roots:                 roots,
		Values:                values,
		LeafLowerRangeValues:  leafLowerRangeValues,
		LeafHigherRangeValues: leafHigherRangeValues,
		LeafIndices:           leafIndices,
		InPathIndices:         inPathIndices,
		InPathElements:        inPathElements,
	}
	return frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &circuit)
}

func SetupNonInclusion(treeDepth uint32, numberOfUtxos uint32) (*ProvingSystem, error) {
	ccs, err := R1CSNonInclusion(treeDepth, numberOfUtxos)
	if err != nil {
		return nil, err
	}
	pk, vk, err := groth16.Setup(ccs)
	if err != nil {
		return nil, err
	}
	return &ProvingSystem{0, 0, treeDepth, numberOfUtxos, pk, vk, ccs}, nil
}

func (ps *ProvingSystem) ProveNonInclusion(params *NonInclusionParameters) (*Proof, error) {
	if err := params.ValidateShape(ps.NonInclusionTreeDepth, ps.NonInclusionNumberOfUtxos); err != nil {
		return nil, err
	}

	roots := make([]frontend.Variable, ps.NonInclusionNumberOfUtxos)
	values := make([]frontend.Variable, ps.NonInclusionNumberOfUtxos)

	leafLowerRangeValues := make([]frontend.Variable, ps.NonInclusionNumberOfUtxos)
	leafHigherRangeValues := make([]frontend.Variable, ps.NonInclusionNumberOfUtxos)
	leafIndices := make([]frontend.Variable, ps.NonInclusionNumberOfUtxos)

	inPathElements := make([][]frontend.Variable, ps.NonInclusionNumberOfUtxos)
	inPathIndices := make([]frontend.Variable, ps.NonInclusionNumberOfUtxos)

	for i := 0; i < int(ps.NonInclusionNumberOfUtxos); i++ {
		roots[i] = params.Roots[i]
		values[i] = params.Values[i]
		leafLowerRangeValues[i] = params.LeafLowerRangeValues[i]
		leafHigherRangeValues[i] = params.LeafHigherRangeValues[i]
		leafIndices[i] = params.LeafIndices[i]
		inPathIndices[i] = params.InPathIndices[i]
		inPathElements[i] = make([]frontend.Variable, ps.NonInclusionTreeDepth)
		for j := 0; j < int(ps.NonInclusionTreeDepth); j++ {
			inPathElements[i][j] = params.InPathElements[i][j]
		}
	}

	assignment := NonInclusionCircuit{
		Roots:                 roots,
		Values:                values,
		LeafLowerRangeValues:  leafLowerRangeValues,
		LeafHigherRangeValues: leafHigherRangeValues,
		LeafIndices:           leafIndices,
		InPathIndices:         inPathIndices,
		InPathElements:        inPathElements,
	}

	witness, err := frontend.NewWitness(&assignment, ecc.BN254.ScalarField())
	if err != nil {
		return nil, err
	}

	logging.Logger().Info().Msg("Proof non-inclusion" + strconv.Itoa(int(ps.NonInclusionTreeDepth)) + " " + strconv.Itoa(int(ps.NonInclusionNumberOfUtxos)))
	proof, err := groth16.Prove(ps.ConstraintSystem, ps.ProvingKey, witness)
	if err != nil {
		logging.Logger().Error().Msg("non-inclusion prove error: " + err.Error())
		return nil, err
	}

	return &Proof{proof}, nil
}
