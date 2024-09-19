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
	"github.com/reilabs/gnark-lean-extractor/v2/abstractor"
)

type InsertionCircuit struct {
	// InputHash frontend.Variable `gnark:",public"`

	// private inputs
	StartIndex frontend.Variable `gnark:"input"`
	PreRoot    frontend.Variable `gnark:",public"`
	PostRoot   frontend.Variable `gnark:",public"`

	Leaves       []frontend.Variable   `gnark:"input"`
	MerkleProofs [][]frontend.Variable `gnark:"input"`

	BatchSize uint32
	Depth     uint32
}

func (circuit *InsertionCircuit) Define(api frontend.API) error {

	// api.AssertIsEqual(circuit.InputHash, circuit.InputHash)

	root := abstractor.Call(api, InsertionProof{
		StartIndex:   circuit.StartIndex,
		PreRoot:      circuit.PreRoot,
		Leaves:       circuit.Leaves,
		MerkleProofs: circuit.MerkleProofs,

		BatchSize: circuit.BatchSize,
		Depth:     circuit.Depth,
	})
	api.AssertIsEqual(root, circuit.PostRoot)

	return nil
}

type InsertionParameters struct {
	PreRoot      big.Int
	PostRoot     big.Int
	StartIndex   uint32
	Leaves       []big.Int
	MerkleProofs [][]big.Int
}

func (p *InsertionParameters) TreeDepth() uint32 {
	if len(p.MerkleProofs) == 0 {
		return 0
	}
	return uint32(len(p.MerkleProofs[0]))
}

func (p *InsertionParameters) BatchSize() uint32 {
	return uint32(len(p.Leaves))
}

func (p *InsertionParameters) ValidateShape(treeDepth uint32, batchSize uint32) error {
	if len(p.Leaves) != int(batchSize) {
		return fmt.Errorf("wrong number of leaves: %d", len(p.Leaves))
	}
	if len(p.MerkleProofs) != int(batchSize) {
		return fmt.Errorf("wrong number of merkle proofs: %d", len(p.MerkleProofs))
	}
	for i, proof := range p.MerkleProofs {
		if len(proof) != int(treeDepth) {
			return fmt.Errorf("wrong size of merkle proof for proof %d: %d", i, len(proof))
		}
		// for j, element := range proof {
		// 	if element.Sign() == 0 {
		// 		return fmt.Errorf("merkle proof element at index [%d][%d] is zero", i, j)
		// 	}
		// }
	}

	// Do we need to check that the leaves are non-zero?
	for i, leaf := range p.Leaves {
		if leaf.Sign() == 0 {
			return fmt.Errorf("leaf at index %d is zero", i)
		}
	}
	return nil
}

func SetupInsertion(treeDepth uint32, batchSize uint32) (*ProvingSystem, error) {
	fmt.Println("Setting up insertion")
	ccs, err := R1CSInsertion(treeDepth, batchSize)
	if err != nil {
		return nil, err
	}
	pk, vk, err := groth16.Setup(ccs)
	if err != nil {
		return nil, err
	}
	return &ProvingSystem{
		InsertionDepth:     treeDepth,
		InsertionBatchSize: batchSize,
		ProvingKey:         pk,
		VerifyingKey:       vk,
		ConstraintSystem:   ccs}, nil
}

func ImportInsertionSetup(treeDepth uint32, batchSize uint32, pkPath string, vkPath string) (*ProvingSystem, error) {
	leaves := make([]frontend.Variable, batchSize)
	merkleProofs := make([][]frontend.Variable, batchSize)

	for i := 0; i < int(batchSize); i++ {
		merkleProofs[i] = make([]frontend.Variable, treeDepth)
	}

	circuit := InsertionCircuit{
		Depth:        treeDepth,
		BatchSize:    batchSize,
		Leaves:       leaves,
		MerkleProofs: merkleProofs,
	}

	fmt.Println("Compiling circuit")
	ccs, err := frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &circuit)
	if err != nil {
		fmt.Println("Error compiling circuit")
		return nil, err
	} else {
		fmt.Println("Compiled circuit successfully")
	}

	pk, err := LoadProvingKey(pkPath)
	if err != nil {
		return nil, err
	}

	vk, err := LoadVerifyingKey(vkPath)
	if err != nil {
		return nil, err
	}

	return &ProvingSystem{
		InclusionTreeDepth: treeDepth,
		InsertionBatchSize: batchSize,
		ProvingKey:         pk,
		VerifyingKey:       vk,
		ConstraintSystem:   ccs,
	}, nil
}

func (ps *ProvingSystem) ProveInsertion(params *InsertionParameters) (*Proof, error) {
	logging.Logger().Info().Interface("params", params).Msg("Received insertion parameters")

	// Log individual fields
	logging.Logger().Info().
		Str("PreRoot", params.PreRoot.String()).
		Str("PostRoot", params.PostRoot.String()).
		Uint32("StartIndex", params.StartIndex).
		Int("LeavesCount", len(params.Leaves)).
		Int("MerkleProofsCount", len(params.MerkleProofs)).
		Msg("Insertion parameters details")

	if err := params.ValidateShape(ps.InsertionDepth, ps.InsertionBatchSize); err != nil {
		return nil, err
	}

	preRoot := frontend.Variable(params.PreRoot.String())
	postRoot := frontend.Variable(params.PostRoot.String())
	startIndex := frontend.Variable(params.StartIndex)
	// inputHash := frontend.Variable("")
	leaves := make([]frontend.Variable, len(params.Leaves))
	for i, leaf := range params.Leaves {
		leaves[i] = frontend.Variable(leaf.String())
	}

	merkleProofs := make([][]frontend.Variable, len(params.MerkleProofs))
	for i, proof := range params.MerkleProofs {
		merkleProofs[i] = make([]frontend.Variable, len(proof))
		for j, element := range proof {
			merkleProofs[i][j] = frontend.Variable(element.String())
		}
	}

	assignment := InsertionCircuit{
		// InputHash:    inputHash,
		PreRoot:      preRoot,
		PostRoot:     postRoot,
		StartIndex:   startIndex,
		Leaves:       leaves,
		MerkleProofs: merkleProofs,
		BatchSize:    ps.InsertionBatchSize,
		Depth:        ps.InsertionDepth,
	}

	fmt.Println("3 ProveInsertion", assignment)
	logging.Logger().Info().Interface("assignment", assignment).Msg("Circuit assignment")
	witness, err := frontend.NewWitness(&assignment, ecc.BN254.ScalarField())
	if err != nil {
		return nil, err
	}

	logging.Logger().Info().Msg("Generating insertion proof " + strconv.Itoa(int(ps.InsertionDepth)) + " " + strconv.Itoa(int(ps.InsertionBatchSize)))
	proof, err := groth16.Prove(ps.ConstraintSystem, ps.ProvingKey, witness)
	if err != nil {
		return nil, err
	}

	return &Proof{proof}, nil
}

func (ps *ProvingSystem) VerifyInsertion(preRoot big.Int, postRoot big.Int, proof *Proof) error {
	publicAssignment := InsertionCircuit{
		PreRoot:  preRoot,
		PostRoot: postRoot,
	}
	witness, err := frontend.NewWitness(&publicAssignment, ecc.BN254.ScalarField(), frontend.PublicOnly())
	if err != nil {
		return err
	}
	return groth16.Verify(proof.Proof, ps.VerifyingKey, witness)
}

func R1CSInsertion(treeDepth uint32, batchSize uint32) (constraint.ConstraintSystem, error) {
	leaves := make([]frontend.Variable, batchSize)
	merkleProofs := make([][]frontend.Variable, batchSize)

	for i := 0; i < int(batchSize); i++ {
		merkleProofs[i] = make([]frontend.Variable, treeDepth)
	}
	circuit := InsertionCircuit{
		Depth:        treeDepth,
		BatchSize:    batchSize,
		Leaves:       leaves,
		MerkleProofs: merkleProofs,
		StartIndex:   frontend.Variable(0),
		PreRoot:      frontend.Variable(0),
		PostRoot:     frontend.Variable(0),
	}
	return frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &circuit)
}

type InsertionProof struct {
	PreRoot      frontend.Variable
	StartIndex   frontend.Variable
	Leaves       []frontend.Variable
	MerkleProofs [][]frontend.Variable

	BatchSize uint32
	Depth     uint32
}

func (gadget InsertionProof) DefineGadget(api frontend.API) interface{} {
	currentRoot := gadget.PreRoot

	for i := 0; i < int(gadget.BatchSize); i++ {
		currentIndex := api.Add(gadget.StartIndex, i)
		currentRoot = abstractor.Call(api, InsertionRound{
			Index:    currentIndex,
			Leaf:     gadget.Leaves[i],
			PrevRoot: currentRoot,
			Proof:    gadget.MerkleProofs[i],
			Depth:    int(gadget.Depth),
		})
	}
	return currentRoot
}

type InsertionRound struct {
	Index    frontend.Variable
	Leaf     frontend.Variable
	PrevRoot frontend.Variable
	Proof    []frontend.Variable
	Depth    int
}

func (gadget InsertionRound) DefineGadget(api frontend.API) interface{} {
	currentPath := api.ToBinary(gadget.Index, gadget.Depth)

	// Verify the leaf was initially empty
	emptyLeaf := frontend.Variable(0)
	root := abstractor.Call(api, VerifyProof{
		Leaf:  emptyLeaf,
		Path:  currentPath,
		Proof: gadget.Proof,
	})
	api.AssertIsEqual(root, gadget.PrevRoot)

	// Insert the new leaf
	newRoot := abstractor.Call(api, VerifyProof{
		Leaf:  gadget.Leaf,
		Path:  currentPath,
		Proof: gadget.Proof,
	})

	return newRoot
}
