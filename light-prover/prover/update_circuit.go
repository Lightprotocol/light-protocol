package prover

import (
	"fmt"
	"math/big"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/constraint"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
	"github.com/reilabs/gnark-lean-extractor/v2/abstractor"
)

type BatchUpdateCircuit struct {
	PreRoot    frontend.Variable `gnark:",public"`
	PostRoot   frontend.Variable `gnark:",public"`
	StartIndex frontend.Variable `gnark:"input"`

	OldLeaves    []frontend.Variable   `gnark:"input"`
	NewLeaves    []frontend.Variable   `gnark:"input"`
	MerkleProofs [][]frontend.Variable `gnark:"input"`

	BatchSize uint32
	Depth     uint32
}

func (circuit *BatchUpdateCircuit) Define(api frontend.API) error {
	root := abstractor.Call(api, BatchUpdateProof{
		StartIndex:   circuit.StartIndex,
		PreRoot:      circuit.PreRoot,
		OldLeaves:    circuit.OldLeaves,
		NewLeaves:    circuit.NewLeaves,
		MerkleProofs: circuit.MerkleProofs,
		BatchSize:    circuit.BatchSize,
		Depth:        circuit.Depth,
	})
	api.AssertIsEqual(root, circuit.PostRoot)

	return nil
}

type BatchUpdateParameters struct {
	PreRoot      big.Int
	PostRoot     big.Int
	StartIndex   uint32
	OldLeaves    []big.Int
	NewLeaves    []big.Int
	MerkleProofs [][]big.Int
}

func (p *BatchUpdateParameters) TreeDepth() uint32 {
	if len(p.MerkleProofs) == 0 {
		return 0
	}
	return uint32(len(p.MerkleProofs[0]))
}

func (p *BatchUpdateParameters) BatchSize() uint32 {
	return uint32(len(p.NewLeaves))
}

func (p *BatchUpdateParameters) ValidateShape(treeDepth uint32, batchSize uint32) error {
	if len(p.OldLeaves) != int(batchSize) || len(p.NewLeaves) != int(batchSize) {
		return fmt.Errorf("wrong number of leaves: old=%d, new=%d, expected=%d", len(p.OldLeaves), len(p.NewLeaves), batchSize)
	}
	if len(p.MerkleProofs) != int(batchSize) {
		return fmt.Errorf("wrong number of merkle proofs: %d", len(p.MerkleProofs))
	}
	for i, proof := range p.MerkleProofs {
		if len(proof) != int(treeDepth) {
			return fmt.Errorf("wrong size of merkle proof for proof %d: %d", i, len(proof))
		}
	}
	return nil
}

func SetupBatchUpdate(treeDepth uint32, batchSize uint32) (*ProvingSystem, error) {
	fmt.Println("Setting up batch update")
	ccs, err := R1CSBatchUpdate(treeDepth, batchSize)
	if err != nil {
		return nil, err
	}
	pk, vk, err := groth16.Setup(ccs)
	if err != nil {
		return nil, err
	}
	return &ProvingSystem{
		UpdateDepth:      treeDepth,
		UpdateBatchSize:  batchSize,
		ProvingKey:       pk,
		VerifyingKey:     vk,
		ConstraintSystem: ccs}, nil
}

func (ps *ProvingSystem) ProveBatchUpdate(params *BatchUpdateParameters) (*Proof, error) {
	if err := params.ValidateShape(ps.UpdateDepth, ps.UpdateBatchSize); err != nil {
		return nil, err
	}

	preRoot := frontend.Variable(params.PreRoot.String())
	postRoot := frontend.Variable(params.PostRoot.String())
	startIndex := frontend.Variable(params.StartIndex)

	oldLeaves := make([]frontend.Variable, len(params.OldLeaves))
	newLeaves := make([]frontend.Variable, len(params.NewLeaves))
	merkleProofs := make([][]frontend.Variable, len(params.MerkleProofs))

	for i := 0; i < len(params.OldLeaves); i++ {
		oldLeaves[i] = frontend.Variable(params.OldLeaves[i].String())
		newLeaves[i] = frontend.Variable(params.NewLeaves[i].String())
		merkleProofs[i] = make([]frontend.Variable, len(params.MerkleProofs[i]))
		for j := 0; j < len(params.MerkleProofs[i]); j++ {
			merkleProofs[i][j] = frontend.Variable(params.MerkleProofs[i][j].String())
		}
	}

	assignment := BatchUpdateCircuit{
		PreRoot:      preRoot,
		PostRoot:     postRoot,
		StartIndex:   startIndex,
		OldLeaves:    oldLeaves,
		NewLeaves:    newLeaves,
		MerkleProofs: merkleProofs,
		BatchSize:    ps.UpdateBatchSize,
		Depth:        ps.UpdateDepth,
	}

	fmt.Printf("Debug - BatchUpdateCircuit: %+v\n", assignment)

	witness, err := frontend.NewWitness(&assignment, ecc.BN254.ScalarField())
	if err != nil {
		return nil, fmt.Errorf("error creating witness: %v", err)
	}

	proof, err := groth16.Prove(ps.ConstraintSystem, ps.ProvingKey, witness)
	if err != nil {
		return nil, fmt.Errorf("error proving: %v", err)
	}

	return &Proof{proof}, nil
}

func (ps *ProvingSystem) VerifyBatchUpdate(preRoot big.Int, postRoot big.Int, proof *Proof) error {
	publicAssignment := BatchUpdateCircuit{
		PreRoot:  preRoot,
		PostRoot: postRoot,
	}
	witness, err := frontend.NewWitness(&publicAssignment, ecc.BN254.ScalarField(), frontend.PublicOnly())
	if err != nil {
		return err
	}
	return groth16.Verify(proof.Proof, ps.VerifyingKey, witness)
}

func R1CSBatchUpdate(treeDepth uint32, batchSize uint32) (constraint.ConstraintSystem, error) {
	oldLeaves := make([]frontend.Variable, batchSize)
	newLeaves := make([]frontend.Variable, batchSize)
	merkleProofs := make([][]frontend.Variable, batchSize)

	for i := 0; i < int(batchSize); i++ {
		merkleProofs[i] = make([]frontend.Variable, treeDepth)
	}
	circuit := BatchUpdateCircuit{
		Depth:        treeDepth,
		BatchSize:    batchSize,
		OldLeaves:    oldLeaves,
		NewLeaves:    newLeaves,
		MerkleProofs: merkleProofs,
		StartIndex:   frontend.Variable(0),
		PreRoot:      frontend.Variable(0),
		PostRoot:     frontend.Variable(0),
	}
	return frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &circuit)
}

type BatchUpdateProof struct {
	PreRoot      frontend.Variable
	PostRoot     frontend.Variable
	StartIndex   frontend.Variable
	OldLeaves    []frontend.Variable
	NewLeaves    []frontend.Variable
	MerkleProofs [][]frontend.Variable

	BatchSize uint32
	Depth     uint32
}

func (gadget BatchUpdateProof) DefineGadget(api frontend.API) interface{} {
	currentRoot := gadget.PreRoot

	fmt.Printf("Debug - BatchUpdateProof: PreRoot: %v\n", gadget.PreRoot)
	fmt.Printf("Debug - BatchUpdateProof: PostRoot: %v\n", gadget.PostRoot)
	fmt.Printf("Debug - BatchUpdateProof: StartIndex: %v\n", gadget.StartIndex)
	fmt.Printf("Debug - BatchUpdateProof: BatchSize: %d\n", gadget.BatchSize)

	for i := 0; i < int(gadget.BatchSize); i++ {
		currentIndex := api.Add(gadget.StartIndex, i)
		fmt.Printf("Debug - BatchUpdateProof: Round %d, CurrentRoot: %v\n", i, currentRoot)

		// Verify the old leaf
		oldRoot := abstractor.Call(api, VerifyProof{
			Leaf:  gadget.OldLeaves[i],
			Path:  api.ToBinary(currentIndex, int(gadget.Depth)),
			Proof: gadget.MerkleProofs[i],
		})
		api.AssertIsEqual(oldRoot, currentRoot)

		// Update with the new leaf
		newRoot := abstractor.Call(api, VerifyProof{
			Leaf:  gadget.NewLeaves[i],
			Path:  api.ToBinary(currentIndex, int(gadget.Depth)),
			Proof: gadget.MerkleProofs[i],
		})
		currentRoot = newRoot
	}
	fmt.Printf("Debug - BatchUpdateProof: Final root: %v, Expected PostRoot: %v\n", currentRoot, gadget.PostRoot)
	api.AssertIsEqual(currentRoot, gadget.PostRoot)

	return currentRoot
}

type BatchUpdateRound struct {
	Index    frontend.Variable
	OldLeaf  frontend.Variable
	NewLeaf  frontend.Variable
	PrevRoot frontend.Variable
	Proof    []frontend.Variable
	Depth    int
}

func (gadget BatchUpdateRound) DefineGadget(api frontend.API) interface{} {
	currentPath := api.ToBinary(gadget.Index, gadget.Depth)

	fmt.Printf("Debug - BatchUpdateRound: Index: %v, Depth: %d\n", gadget.Index, gadget.Depth)
	fmt.Printf("Debug - BatchUpdateRound: OldLeaf: %v\n", gadget.OldLeaf)
	fmt.Printf("Debug - BatchUpdateRound: NewLeaf: %v\n", gadget.NewLeaf)
	fmt.Printf("Debug - BatchUpdateRound: PrevRoot: %v\n", gadget.PrevRoot)
	fmt.Printf("Debug - BatchUpdateRound: Proof: %v\n", gadget.Proof)

	// Update with the new leaf
	newRoot := abstractor.Call(api, VerifyProof{
		Leaf:  gadget.NewLeaf,
		Path:  currentPath,
		Proof: gadget.Proof,
	})

	fmt.Printf("Debug - BatchUpdateRound: New root: %v\n", newRoot)

	return newRoot
}

func ImportBatchUpdateSetup(treeDepth uint32, batchSize uint32, pkPath string, vkPath string) (*ProvingSystem, error) {
	oldLeaves := make([]frontend.Variable, batchSize)
	newLeaves := make([]frontend.Variable, batchSize)
	merkleProofs := make([][]frontend.Variable, batchSize)

	for i := 0; i < int(batchSize); i++ {
		merkleProofs[i] = make([]frontend.Variable, treeDepth)
	}

	circuit := BatchUpdateCircuit{
		Depth:        treeDepth,
		BatchSize:    batchSize,
		OldLeaves:    oldLeaves,
		NewLeaves:    newLeaves,
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
		UpdateDepth:      treeDepth,
		UpdateBatchSize:  batchSize,
		ProvingKey:       pk,
		VerifyingKey:     vk,
		ConstraintSystem: ccs,
	}, nil
}
