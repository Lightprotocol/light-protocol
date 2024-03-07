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
	Root  []big.Int
	Value []big.Int

	LeafLowerRangeValue  []big.Int
	LeafHigherRangeValue []big.Int
	LeafIndex            []uint32

	InPathIndices  []uint32
	InPathElements [][]big.Int
}

func (p *NonInclusionParameters) NumberOfUTXOs() uint32 {
	return uint32(len(p.Root))
}

func (p *NonInclusionParameters) TreeDepth() uint32 {
	if len(p.InPathElements) == 0 {
		return 0
	}
	return uint32(len(p.InPathElements[0]))
}

func (p *NonInclusionParameters) ValidateShape(treeDepth uint32, numOfUTXOs uint32) error {
	if p.NumberOfUTXOs() != numOfUTXOs {
		return fmt.Errorf("wrong number of utxos: %d", len(p.Root))
	}
	if p.TreeDepth() != treeDepth {
		return fmt.Errorf("wrong size of merkle proof for proof %d: %d", p.NumberOfUTXOs(), p.TreeDepth())
	}
	return nil
}

func R1CSNonInclusion(treeDepth uint32, numberOfUtxos uint32) (constraint.ConstraintSystem, error) {
	root := make([]frontend.Variable, numberOfUtxos)
	value := make([]frontend.Variable, numberOfUtxos)

	leafLowerRangeValue := make([]frontend.Variable, numberOfUtxos)
	leafHigherRangeValue := make([]frontend.Variable, numberOfUtxos)
	leafIndex := make([]frontend.Variable, numberOfUtxos)

	inPathIndices := make([]frontend.Variable, numberOfUtxos)
	inPathElements := make([][]frontend.Variable, numberOfUtxos)

	for i := 0; i < int(numberOfUtxos); i++ {
		inPathElements[i] = make([]frontend.Variable, treeDepth)
	}

	circuit := NonInclusionCircuit{
		Depth:                int(treeDepth),
		NumberOfUtxos:        int(numberOfUtxos),
		Root:                 root,
		Value:                value,
		LeafLowerRangeValue:  leafLowerRangeValue,
		LeafHigherRangeValue: leafHigherRangeValue,
		LeafIndex:            leafIndex,
		InPathIndices:        inPathIndices,
		InPathElements:       inPathElements,
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
	return &ProvingSystem{treeDepth, numberOfUtxos, false, pk, vk, ccs}, nil
}

func (ps *ProvingSystem) ProveNonInclusion(params *NonInclusionParameters) (*Proof, error) {
	if err := params.ValidateShape(ps.TreeDepth, ps.NumberOfUtxos); err != nil {
		return nil, err
	}

	root := make([]frontend.Variable, ps.NumberOfUtxos)
	value := make([]frontend.Variable, ps.NumberOfUtxos)

	leafLowerRangeValue := make([]frontend.Variable, ps.NumberOfUtxos)
	leafHigherRangeValue := make([]frontend.Variable, ps.NumberOfUtxos)
	leafIndex := make([]frontend.Variable, ps.NumberOfUtxos)

	inPathElements := make([][]frontend.Variable, ps.NumberOfUtxos)
	inPathIndices := make([]frontend.Variable, ps.NumberOfUtxos)

	for i := 0; i < int(ps.NumberOfUtxos); i++ {
		root[i] = params.Root[i]
		value[i] = params.Value[i]
		leafLowerRangeValue[i] = params.LeafLowerRangeValue[i]
		leafHigherRangeValue[i] = params.LeafHigherRangeValue[i]
		leafIndex[i] = params.LeafIndex[i]
		inPathIndices[i] = params.InPathIndices[i]
		inPathElements[i] = make([]frontend.Variable, ps.TreeDepth)
		for j := 0; j < int(ps.TreeDepth); j++ {
			inPathElements[i][j] = params.InPathElements[i][j]
		}
	}

	fmt.Println("root", root)
	fmt.Println("value", value)
	fmt.Println("leafLowerRangeValue", leafLowerRangeValue)
	fmt.Println("leafHigherRangeValue", leafHigherRangeValue)
	fmt.Println("leafIndex", leafIndex)

	assignment := NonInclusionCircuit{
		Root:                 root,
		Value:                value,
		LeafLowerRangeValue:  leafLowerRangeValue,
		LeafHigherRangeValue: leafHigherRangeValue,
		LeafIndex:            leafIndex,
		InPathIndices:        inPathIndices,
		InPathElements:       inPathElements,
	}

	witness, err := frontend.NewWitness(&assignment, ecc.BN254.ScalarField())
	if err != nil {
		return nil, err
	}

	logging.Logger().Info().Msg("Proof non-inclusion" + strconv.Itoa(int(ps.TreeDepth)) + " " + strconv.Itoa(int(ps.NumberOfUtxos)))
	proof, err := groth16.Prove(ps.ConstraintSystem, ps.ProvingKey, witness)
	if err != nil {
		logging.Logger().Error().Msg("non-inclusion prove error: " + err.Error())
		return nil, err
	}

	return &Proof{proof}, nil
}

func (ps *ProvingSystem) VerifyNonInclusion(root []big.Int, value []big.Int, proof *Proof) error {
	rootArray := make([]frontend.Variable, ps.NumberOfUtxos)
	for i, v := range root {
		rootArray[i] = v
	}

	valueArray := make([]frontend.Variable, ps.NumberOfUtxos)
	for i, v := range value {
		valueArray[i] = v
	}

	publicAssignment := NonInclusionCircuit{
		Root:  rootArray,
		Value: valueArray,
	}
	witness, err := frontend.NewWitness(&publicAssignment, ecc.BN254.ScalarField(), frontend.PublicOnly())
	if err != nil {
		return err
	}
	return groth16.Verify(proof.Proof, ps.VerifyingKey, witness)
}
