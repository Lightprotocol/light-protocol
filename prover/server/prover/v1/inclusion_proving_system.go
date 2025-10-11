package v1

import (
	"fmt"
	"light/light-prover/logging"
	"light/light-prover/prover/common"
	"math/big"
	"strconv"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/frontend"
)

type InclusionInputs struct {
	Root         big.Int
	PathIndex    uint32
	PathElements []big.Int
	Leaf         big.Int
}

type InclusionParameters struct {
	Inputs []InclusionInputs
}

func (p *InclusionParameters) NumberOfCompressedAccounts() uint32 {
	return uint32(len(p.Inputs))
}

func (p *InclusionParameters) TreeHeight() uint32 {
	if len(p.Inputs) == 0 {
		return 0
	}
	return uint32(len(p.Inputs[0].PathElements))
}

func (p *InclusionParameters) ValidateShape(treeHeight uint32, numOfCompressedAccounts uint32) error {
	if p.NumberOfCompressedAccounts() != numOfCompressedAccounts {
		return fmt.Errorf("wrong number of compressed accounts: %d", p.NumberOfCompressedAccounts())
	}
	if p.TreeHeight() != treeHeight {
		return fmt.Errorf("wrong size of merkle proof for proof %d: %d", p.NumberOfCompressedAccounts(), p.TreeHeight())
	}
	return nil
}

func ProveInclusion(ps *common.MerkleProofSystem, params *InclusionParameters) (*common.Proof, error) {
	if err := params.ValidateShape(ps.InclusionTreeHeight, ps.InclusionNumberOfCompressedAccounts); err != nil {
		return nil, err
	}

	inPathIndices := make([]frontend.Variable, ps.InclusionNumberOfCompressedAccounts)
	roots := make([]frontend.Variable, ps.InclusionNumberOfCompressedAccounts)
	leaves := make([]frontend.Variable, ps.InclusionNumberOfCompressedAccounts)
	inPathElements := make([][]frontend.Variable, ps.InclusionNumberOfCompressedAccounts)

	for i := 0; i < int(ps.InclusionNumberOfCompressedAccounts); i++ {
		roots[i] = params.Inputs[i].Root
		leaves[i] = params.Inputs[i].Leaf
		inPathIndices[i] = params.Inputs[i].PathIndex
		inPathElements[i] = make([]frontend.Variable, ps.InclusionTreeHeight)
		for j := 0; j < int(ps.InclusionTreeHeight); j++ {
			inPathElements[i][j] = params.Inputs[i].PathElements[j]
		}
	}

	assignment := InclusionCircuit{
		Roots:                      roots,
		Leaves:                     leaves,
		InPathIndices:              inPathIndices,
		InPathElements:             inPathElements,
		NumberOfCompressedAccounts: ps.InclusionNumberOfCompressedAccounts,
		Height:                     ps.InclusionTreeHeight,
	}

	witness, err := frontend.NewWitness(&assignment, ecc.BN254.ScalarField())
	if err != nil {
		return nil, err
	}

	logging.Logger().Info().Msg("Proof inclusion" + strconv.Itoa(int(ps.InclusionTreeHeight)) + " " + strconv.Itoa(int(ps.InclusionNumberOfCompressedAccounts)))
	proof, err := groth16.Prove(ps.ConstraintSystem, ps.ProvingKey, witness)
	if err != nil {
		return nil, err
	}

	return &common.Proof{Proof: proof}, nil
}
