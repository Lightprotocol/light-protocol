package prover

import (
	"fmt"
	merkle_tree "light/light-prover/merkle-tree"
	"math/big"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/constraint"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
	"github.com/reilabs/gnark-lean-extractor/v2/abstractor"
)

type BatchAppend2Circuit struct {
	PublicInputHash     frontend.Variable `gnark:",public"`
	OldRoot             frontend.Variable `gnark:",private"`
	NewRoot             frontend.Variable `gnark:",private"`
	LeavesHashchainHash frontend.Variable `gnark:",private"`
	StartIndex          frontend.Variable `gnark:",private"`

	OldLeaves    []frontend.Variable   `gnark:",private"`
	Leaves       []frontend.Variable   `gnark:",private"`
	MerkleProofs [][]frontend.Variable `gnark:",private"`

	Height    uint32
	BatchSize uint32
}

func (circuit *BatchAppend2Circuit) Define(api frontend.API) error {

	hashChainInputs := make([]frontend.Variable, int(4))
	hashChainInputs[0] = circuit.OldRoot
	hashChainInputs[1] = circuit.NewRoot
	hashChainInputs[2] = circuit.LeavesHashchainHash
	hashChainInputs[3] = circuit.StartIndex

	publicInputsHashChain := createHashChain(api, int(4), hashChainInputs)
	api.AssertIsEqual(publicInputsHashChain, circuit.PublicInputHash)
	newLeaves := make([]frontend.Variable, int(circuit.BatchSize))
	for i := 0; i < int(circuit.BatchSize); i++ {
		// old leaves is either 0 or a nullifier hash
		// 1. 0 (zero value) means that the leaf has not been spent
		// 		-> insert the new leaf
		// 2. nullifier hash (non-zero) means that the leaf has been spent
		// 		-> keep the old leaf don't insert the new leaf
		selector := api.IsZero(circuit.OldLeaves[i])
		newLeaves[i] = api.Select(selector, circuit.Leaves[i], circuit.OldLeaves[i])
	}

	leavesHashchainHash := createHashChain(api, int(circuit.BatchSize), circuit.Leaves)
	api.AssertIsEqual(leavesHashchainHash, circuit.LeavesHashchainHash)

	newRoot := circuit.OldRoot
	indexBits := api.ToBinary(circuit.StartIndex, int(circuit.Height))
	for i := 0; i < int(circuit.BatchSize); i++ {
		newRoot = abstractor.Call(api, MerkleRootUpdateGadget{
			OldRoot:     newRoot,
			OldLeaf:     circuit.OldLeaves[i],
			NewLeaf:     newLeaves[i],
			PathIndex:   indexBits,
			MerkleProof: circuit.MerkleProofs[i],
			Height:      int(circuit.Height),
		})
		indexBits = incrementBits(api, indexBits)
	}

	api.AssertIsEqual(newRoot, circuit.NewRoot)

	return nil
}

type BatchAppend2Parameters struct {
	PublicInputHash     *big.Int
	OldRoot             *big.Int
	NewRoot             *big.Int
	OldLeaves           []*big.Int
	LeavesHashchainHash *big.Int
	Leaves              []*big.Int
	MerkleProofs        [][]big.Int
	StartIndex          uint32
	Height              uint32
	BatchSize           uint32
	Tree                *merkle_tree.PoseidonTree
}

func (p *BatchAppend2Parameters) TreeDepth() uint32 {
	if len(p.MerkleProofs) == 0 {
		return 0
	}
	return uint32(len(p.MerkleProofs[0]))
}

func (p *BatchAppend2Parameters) ValidateShape() error {
	if len(p.Leaves) != int(p.BatchSize) {
		return fmt.Errorf("wrong number of leaves: %d, expected: %d", len(p.Leaves), p.BatchSize)
	}
	if len(p.OldLeaves) != int(p.BatchSize) {
		return fmt.Errorf("wrong number of tx hashes: %d, expected: %d", len(p.OldLeaves), p.BatchSize)
	}
	if len(p.MerkleProofs) != int(p.BatchSize) {
		return fmt.Errorf("wrong number of merkle proofs: %d", len(p.MerkleProofs))
	}
	for i, proof := range p.MerkleProofs {
		if len(proof) != int(p.Height) {
			return fmt.Errorf("wrong size of merkle proof for proof %d: %d", i, len(proof))
		}
	}
	return nil
}

func SetupBatchAppend2(height uint32, batchSize uint32) (*ProvingSystemV2, error) {
	fmt.Println("Setting up batch update")
	ccs, err := R1CSBatchAppend2(height, batchSize)
	if err != nil {
		return nil, err
	}
	pk, vk, err := groth16.Setup(ccs)
	if err != nil {
		return nil, err
	}
	return &ProvingSystemV2{
		TreeHeight:       height,
		BatchSize:        batchSize,
		ProvingKey:       pk,
		VerifyingKey:     vk,
		ConstraintSystem: ccs}, nil
}

func (ps *ProvingSystemV2) ProveBatchAppend2(params *BatchAppend2Parameters) (*Proof, error) {
	if err := params.ValidateShape(); err != nil {
		return nil, err
	}

	publicInputHash := frontend.Variable(params.PublicInputHash)
	oldRoot := frontend.Variable(params.OldRoot)
	newRoot := frontend.Variable(params.NewRoot)
	leavesHashchainHash := frontend.Variable(params.LeavesHashchainHash)

	txHashes := make([]frontend.Variable, len(params.OldLeaves))
	leaves := make([]frontend.Variable, len(params.Leaves))
	merkleProofs := make([][]frontend.Variable, len(params.MerkleProofs))
	startIndex := params.StartIndex

	for i := 0; i < len(params.Leaves); i++ {
		leaves[i] = frontend.Variable(params.Leaves[i])
		txHashes[i] = frontend.Variable(params.OldLeaves[i])
		merkleProofs[i] = make([]frontend.Variable, len(params.MerkleProofs[i]))
		for j := 0; j < len(params.MerkleProofs[i]); j++ {
			merkleProofs[i][j] = frontend.Variable(params.MerkleProofs[i][j])
		}
	}

	assignment := BatchAppend2Circuit{
		PublicInputHash:     publicInputHash,
		OldRoot:             oldRoot,
		NewRoot:             newRoot,
		OldLeaves:           txHashes,
		LeavesHashchainHash: leavesHashchainHash,
		Leaves:              leaves,
		StartIndex:          startIndex,
		MerkleProofs:        merkleProofs,
		Height:              ps.TreeHeight,
		BatchSize:           ps.BatchSize,
	}

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

func R1CSBatchAppend2(height uint32, batchSize uint32) (constraint.ConstraintSystem, error) {
	leaves := make([]frontend.Variable, batchSize)
	txHashes := make([]frontend.Variable, batchSize)
	pathIndices := make([]frontend.Variable, batchSize)
	merkleProofs := make([][]frontend.Variable, batchSize)

	for i := 0; i < int(batchSize); i++ {
		merkleProofs[i] = make([]frontend.Variable, height)
	}

	circuit := BatchAppend2Circuit{
		PublicInputHash:     frontend.Variable(0),
		OldRoot:             frontend.Variable(0),
		NewRoot:             frontend.Variable(0),
		OldLeaves:           txHashes,
		LeavesHashchainHash: frontend.Variable(0),
		Leaves:              leaves,
		StartIndex:          pathIndices,
		MerkleProofs:        merkleProofs,
		Height:              height,
		BatchSize:           batchSize,
	}

	return frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &circuit)
}

func ImportBatchAppend2Setup(treeHeight uint32, batchSize uint32, pkPath string, vkPath string) (*ProvingSystemV2, error) {
	leaves := make([]frontend.Variable, batchSize)
	txHashes := make([]frontend.Variable, batchSize)
	oldMerkleProofs := make([][]frontend.Variable, batchSize)
	newMerkleProofs := make([][]frontend.Variable, batchSize)

	for i := 0; i < int(batchSize); i++ {
		oldMerkleProofs[i] = make([]frontend.Variable, treeHeight)
		newMerkleProofs[i] = make([]frontend.Variable, treeHeight)
	}

	circuit := BatchAppend2Circuit{
		Height:              treeHeight,
		OldLeaves:           txHashes,
		Leaves:              leaves,
		MerkleProofs:        newMerkleProofs,
		StartIndex:          make([]frontend.Variable, batchSize),
		OldRoot:             frontend.Variable(0),
		NewRoot:             frontend.Variable(0),
		LeavesHashchainHash: frontend.Variable(0),
		BatchSize:           batchSize,
		PublicInputHash:     frontend.Variable(0),
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

	return &ProvingSystemV2{
		TreeHeight:       treeHeight,
		BatchSize:        batchSize,
		ProvingKey:       pk,
		VerifyingKey:     vk,
		ConstraintSystem: ccs,
	}, nil
}
