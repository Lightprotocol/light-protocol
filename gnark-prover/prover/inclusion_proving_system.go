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

type InclusionParameters struct {
	Root           []big.Int
	InPathIndices  []uint32
	InPathElements [][]big.Int
	Leaf           []big.Int
}

func (p *InclusionParameters) NumberOfUTXOs() uint32 {
	return uint32(len(p.Root))
}

func (p *InclusionParameters) TreeDepth() uint32 {
	if len(p.InPathElements) == 0 {
		return 0
	}
	return uint32(len(p.InPathElements[0]))
}

func (p *InclusionParameters) ValidateShape(treeDepth uint32, numOfUTXOs uint32) error {
	if p.NumberOfUTXOs() != numOfUTXOs {
		return fmt.Errorf("wrong number of utxos: %d", len(p.Root))
	}
	if p.TreeDepth() != treeDepth {
		return fmt.Errorf("wrong size of merkle proof for proof %d: %d", p.NumberOfUTXOs(), p.TreeDepth())
	}
	return nil
}

func R1CSInclusion(treeDepth uint32, numberOfUtxos uint32) (constraint.ConstraintSystem, error) {
	root := make([]frontend.Variable, numberOfUtxos)
	leaf := make([]frontend.Variable, numberOfUtxos)
	inPathIndices := make([]frontend.Variable, numberOfUtxos)
	inPathElements := make([][]frontend.Variable, numberOfUtxos)

	for i := 0; i < int(numberOfUtxos); i++ {
		inPathElements[i] = make([]frontend.Variable, treeDepth)
	}

	circuit := InclusionCircuit{
		Depth:          int(treeDepth),
		NumberOfUtxos:  int(numberOfUtxos),
		Root:           root,
		Leaf:           leaf,
		InPathIndices:  inPathIndices,
		InPathElements: inPathElements,
	}
	return frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &circuit)
}

func SetupInclusion(treeDepth uint32, numberOfUtxos uint32) (*ProvingSystem, error) {
	ccs, err := R1CSInclusion(treeDepth, numberOfUtxos)
	if err != nil {
		return nil, err
	}
	pk, vk, err := groth16.Setup(ccs)
	if err != nil {
		return nil, err
	}
	return &ProvingSystem{treeDepth, numberOfUtxos, 0, 0, pk, vk, ccs}, nil
}

func (ps *ProvingSystem) ProveInclusion(params *InclusionParameters) (*Proof, error) {
	if err := params.ValidateShape(ps.InclusionTreeDepth, ps.InclusionNumberOfUtxos); err != nil {
		return nil, err
	}

	inPathIndices := make([]frontend.Variable, ps.InclusionNumberOfUtxos)
	root := make([]frontend.Variable, ps.InclusionNumberOfUtxos)
	leaf := make([]frontend.Variable, ps.InclusionNumberOfUtxos)
	inPathElements := make([][]frontend.Variable, ps.InclusionNumberOfUtxos)

	for i := 0; i < int(ps.InclusionNumberOfUtxos); i++ {
		root[i] = params.Root[i]
		leaf[i] = params.Leaf[i]
		inPathIndices[i] = params.InPathIndices[i]
		inPathElements[i] = make([]frontend.Variable, ps.InclusionTreeDepth)
		for j := 0; j < int(ps.InclusionTreeDepth); j++ {
			inPathElements[i][j] = params.InPathElements[i][j]
		}
	}

	assignment := InclusionCircuit{
		Root:           root,
		Leaf:           leaf,
		InPathIndices:  inPathIndices,
		InPathElements: inPathElements,
	}

	witness, err := frontend.NewWitness(&assignment, ecc.BN254.ScalarField())
	if err != nil {
		return nil, err
	}

	logging.Logger().Info().Msg("Proof inclusion" + strconv.Itoa(int(ps.InclusionTreeDepth)) + " " + strconv.Itoa(int(ps.InclusionNumberOfUtxos)))
	proof, err := groth16.Prove(ps.ConstraintSystem, ps.ProvingKey, witness)
	if err != nil {
		return nil, err
	}

	return &Proof{proof}, nil
}

func (ps *ProvingSystem) VerifyInclusion(root []big.Int, leaf []big.Int, proof *Proof) error {
	leafArray := make([]frontend.Variable, ps.InclusionNumberOfUtxos)
	for i, v := range leaf {
		leafArray[i] = v
	}

	rootArray := make([]frontend.Variable, ps.InclusionNumberOfUtxos)
	for i, v := range root {
		rootArray[i] = v
	}

	publicAssignment := InclusionCircuit{
		Leaf: leafArray,
		Root: rootArray,
	}
	witness, err := frontend.NewWitness(&publicAssignment, ecc.BN254.ScalarField(), frontend.PublicOnly())
	if err != nil {
		return err
	}
	return groth16.Verify(proof.Proof, ps.VerifyingKey, witness)
}
