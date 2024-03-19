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

func (p *InclusionParameters) ToJSON() string {
	// convert params to string of hex values like: {"root":["0x0"],"inPathIndices":[0],"inPathElements":[["0x0"]],"leaf":["0x0"]}
	// create json string variable
	var jsonStr string
	jsonStr += fmt.Sprintf("{\"root\": [")
	// convert root to string of hex values
	for i, v := range p.Root {
		jsonStr += fmt.Sprintf("\"0x%s\"", v.Text(16))
		if i < len(p.Root)-1 {
			jsonStr += ","
		}
	}
	jsonStr += fmt.Sprintf("],")

	jsonStr += fmt.Sprintf("\"inPathIndices\": [")
	// convert inPathIndices to string of uint32 values
	for i, v := range p.InPathIndices {
		jsonStr += fmt.Sprintf("%d", v)
		if i < len(p.InPathIndices)-1 {
			jsonStr += ","
		}
	}
	jsonStr += fmt.Sprintf("],")

	jsonStr += fmt.Sprintf("\"inPathElements\": [")
	// convert inPathElements to string of array of hex values
	for i, v := range p.InPathElements {
		jsonStr += "["
		for j, w := range v {
			jsonStr += fmt.Sprintf("\"0x%s\"", w.Text(16))
			if j < len(v)-1 {
				jsonStr += ","
			}
		}
		jsonStr += "]"
		if i < len(p.InPathElements)-1 {
			jsonStr += ","
		}
	}
	jsonStr += fmt.Sprintf("],")

	jsonStr += fmt.Sprintf("\"leaf\": [")
	// convert leaf to string of hex values
	for i, v := range p.Leaf {
		jsonStr += fmt.Sprintf("\"0x%s\"", v.Text(16))
		if i < len(p.Leaf)-1 {
			jsonStr += ","
		}
	}
	jsonStr += fmt.Sprintf("]}")

	return jsonStr
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
	return &ProvingSystem{treeDepth, numberOfUtxos, true, pk, vk, ccs}, nil
}

func (ps *ProvingSystem) ProveInclusion(params *InclusionParameters) (*Proof, error) {
	if err := params.ValidateShape(ps.TreeDepth, ps.NumberOfUtxos); err != nil {
		return nil, err
	}

	inPathIndices := make([]frontend.Variable, ps.NumberOfUtxos)
	root := make([]frontend.Variable, ps.NumberOfUtxos)
	leaf := make([]frontend.Variable, ps.NumberOfUtxos)
	inPathElements := make([][]frontend.Variable, ps.NumberOfUtxos)

	for i := 0; i < int(ps.NumberOfUtxos); i++ {
		root[i] = params.Root[i]
		leaf[i] = params.Leaf[i]
		inPathIndices[i] = params.InPathIndices[i]
		inPathElements[i] = make([]frontend.Variable, ps.TreeDepth)
		for j := 0; j < int(ps.TreeDepth); j++ {
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

	logging.Logger().Info().Msg("Proof inclusion" + strconv.Itoa(int(ps.TreeDepth)) + " " + strconv.Itoa(int(ps.NumberOfUtxos)))
	proof, err := groth16.Prove(ps.ConstraintSystem, ps.ProvingKey, witness)
	if err != nil {
		return nil, err
	}

	return &Proof{proof}, nil
}

func (ps *ProvingSystem) VerifyInclusion(root []big.Int, leaf []big.Int, proof *Proof) error {
	leafArray := make([]frontend.Variable, ps.NumberOfUtxos)
	for i, v := range leaf {
		leafArray[i] = v
	}

	rootArray := make([]frontend.Variable, ps.NumberOfUtxos)
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
