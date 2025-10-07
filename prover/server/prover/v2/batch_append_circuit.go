package v2

import (
	"fmt"
	merkle_tree "light/light-prover/merkle-tree"
	"light/light-prover/prover/common"
	"math/big"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/constraint"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
	"github.com/reilabs/gnark-lean-extractor/v3/abstractor"
)

type BatchAppendCircuit struct {
	PublicInputHash     frontend.Variable `gnark:",public"`
	OldRoot             frontend.Variable `gnark:",secret"`
	NewRoot             frontend.Variable `gnark:",secret"`
	LeavesHashchainHash frontend.Variable `gnark:",secret"`
	StartIndex          frontend.Variable `gnark:",secret"`

	OldLeaves    []frontend.Variable   `gnark:",secret"`
	Leaves       []frontend.Variable   `gnark:",secret"`
	MerkleProofs [][]frontend.Variable `gnark:",secret"`

	Height    uint32
	BatchSize uint32
}

func (circuit *BatchAppendCircuit) Define(api frontend.API) error {

	hashChainInputs := make([]frontend.Variable, int(4))
	hashChainInputs[0] = circuit.OldRoot
	hashChainInputs[1] = circuit.NewRoot
	hashChainInputs[2] = circuit.LeavesHashchainHash
	hashChainInputs[3] = circuit.StartIndex

	publicInputsHashChain := createHashChain(api, hashChainInputs)
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

	leavesHashchainHash := createHashChain(api, circuit.Leaves)
	api.AssertIsEqual(leavesHashchainHash, circuit.LeavesHashchainHash)

	newRoot := circuit.OldRoot

	for i := 0; i < int(circuit.BatchSize); i++ {
		indexBits := api.ToBinary(api.Add(circuit.StartIndex, i), int(circuit.Height))
		newRoot = abstractor.Call(api, common.MerkleRootUpdateGadget{
			OldRoot:     newRoot,
			OldLeaf:     circuit.OldLeaves[i],
			NewLeaf:     newLeaves[i],
			PathIndex:   indexBits,
			MerkleProof: circuit.MerkleProofs[i],
			Height:      int(circuit.Height),
		})
	}

	api.AssertIsEqual(newRoot, circuit.NewRoot)

	return nil
}

type BatchAppendParameters struct {
	PublicInputHash     *big.Int
	OldRoot             *big.Int
	NewRoot             *big.Int
	OldLeaves           []*big.Int
	LeavesHashchainHash *big.Int
	Leaves              []*big.Int
	MerkleProofs        [][]big.Int
	StartIndex          uint64
	Height              uint32
	BatchSize           uint32
	Tree                *merkle_tree.PoseidonTree
}

func (p *BatchAppendParameters) ValidateShape() error {
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

func SetupBatchAppend(height uint32, batchSize uint32) (*common.BatchProofSystem, error) {
	fmt.Println("Setting up batch update")
	ccs, err := R1CSBatchAppend(height, batchSize)
	if err != nil {
		return nil, err
	}
	pk, vk, err := groth16.Setup(ccs)
	if err != nil {
		return nil, err
	}
	return &common.BatchProofSystem{
		TreeHeight:       height,
		BatchSize:        batchSize,
		ProvingKey:       pk,
		VerifyingKey:     vk,
		ConstraintSystem: ccs}, nil
}

func ProveBatchAppend(ps *common.BatchProofSystem, params *BatchAppendParameters) (*common.Proof, error) {
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

	assignment := BatchAppendCircuit{
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

	return &common.Proof{proof}, nil
}

func R1CSBatchAppend(height uint32, batchSize uint32) (constraint.ConstraintSystem, error) {
	leaves := make([]frontend.Variable, batchSize)
	txHashes := make([]frontend.Variable, batchSize)
	pathIndices := make([]frontend.Variable, batchSize)
	merkleProofs := make([][]frontend.Variable, batchSize)

	for i := 0; i < int(batchSize); i++ {
		merkleProofs[i] = make([]frontend.Variable, height)
	}

	circuit := BatchAppendCircuit{
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

func ImportBatchAppendSetup(treeHeight uint32, batchSize uint32, pkPath string, vkPath string) (*common.BatchProofSystem, error) {
	leaves := make([]frontend.Variable, batchSize)
	txHashes := make([]frontend.Variable, batchSize)
	oldMerkleProofs := make([][]frontend.Variable, batchSize)
	newMerkleProofs := make([][]frontend.Variable, batchSize)

	for i := 0; i < int(batchSize); i++ {
		oldMerkleProofs[i] = make([]frontend.Variable, treeHeight)
		newMerkleProofs[i] = make([]frontend.Variable, treeHeight)
	}

	circuit := BatchAppendCircuit{
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

	pk, err := common.LoadProvingKey(pkPath)
	if err != nil {
		return nil, err
	}

	vk, err := common.LoadVerifyingKey(vkPath)
	if err != nil {
		return nil, err
	}

	return &common.BatchProofSystem{
		TreeHeight:       treeHeight,
		BatchSize:        batchSize,
		ProvingKey:       pk,
		VerifyingKey:     vk,
		ConstraintSystem: ccs,
	}, nil
}
