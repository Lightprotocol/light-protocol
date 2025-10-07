package v2

import (
	"fmt"
	merkle_tree "light/light-prover/merkle-tree"
	"light/light-prover/prover/common"
	"light/light-prover/prover/poseidon"
	"math/big"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/constraint"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
	"github.com/reilabs/gnark-lean-extractor/v3/abstractor"
)

type BatchUpdateCircuit struct {
	PublicInputHash     frontend.Variable `gnark:",public"`
	OldRoot             frontend.Variable `gnark:",secret"`
	NewRoot             frontend.Variable `gnark:",secret"`
	LeavesHashchainHash frontend.Variable `gnark:",secret"`

	TxHashes     []frontend.Variable   `gnark:",secret"`
	Leaves       []frontend.Variable   `gnark:",secret"`
	OldLeaves    []frontend.Variable   `gnark:",secret"`
	MerkleProofs [][]frontend.Variable `gnark:",secret"`
	PathIndices  []frontend.Variable   `gnark:",secret"`

	Height    uint32
	BatchSize uint32
}

func (circuit *BatchUpdateCircuit) Define(api frontend.API) error {

	hashChainInputs := make([]frontend.Variable, int(3))
	hashChainInputs[0] = circuit.OldRoot
	hashChainInputs[1] = circuit.NewRoot
	hashChainInputs[2] = circuit.LeavesHashchainHash
	publicInputsHashChain := createHashChain(api, hashChainInputs)
	api.AssertIsEqual(publicInputsHashChain, circuit.PublicInputHash)
	nullifiers := make([]frontend.Variable, int(circuit.BatchSize))
	// We might nullify leaves which have not been appended yet. Hence we need
	// to handle the case where the OldLeaf is 0 and not equal to leaves[i].
	// Leaves[i] is checked as part of the nullifier hash
	// path index is checked as part of the nullifier hash
	//	in case old leaf is 0 the checked path index
	//	ensures the correct leaf is nullified (leaf hash depends on the leaf index)
	// old leaf is checked with the initial Merkle proof
	//	which is checked against the onchain root
	for i := 0; i < int(circuit.BatchSize); i++ {
		// - We need to include path index in the nullifier hash so
		// that it is checked in case OldLeaf is 0 -> Leaves[i] is not inserted
		// yet but has to be inserted into a specific index
		nullifiers[i] = abstractor.Call(api, poseidon.Poseidon3{In1: circuit.Leaves[i], In2: circuit.PathIndices[i], In3: circuit.TxHashes[i]})
	}

	nullifierHashChainHash := createHashChain(api, nullifiers)
	api.AssertIsEqual(nullifierHashChainHash, circuit.LeavesHashchainHash)

	newRoot := circuit.OldRoot

	for i := 0; i < int(circuit.BatchSize); i++ {
		currentPath := api.ToBinary(circuit.PathIndices[i], int(circuit.Height))
		newRoot = abstractor.Call(api, common.MerkleRootUpdateGadget{
			OldRoot:     newRoot,
			OldLeaf:     circuit.OldLeaves[i],
			NewLeaf:     nullifiers[i],
			PathIndex:   currentPath,
			MerkleProof: circuit.MerkleProofs[i],
			Height:      int(circuit.Height),
		})
	}

	api.AssertIsEqual(newRoot, circuit.NewRoot)

	return nil
}

type BatchUpdateParameters struct {
	PublicInputHash     *big.Int
	OldRoot             *big.Int
	NewRoot             *big.Int
	TxHashes            []*big.Int
	LeavesHashchainHash *big.Int
	Leaves              []*big.Int
	OldLeaves           []*big.Int
	MerkleProofs        [][]big.Int
	PathIndices         []uint32
	Height              uint32
	BatchSize           uint32
	Tree                *merkle_tree.PoseidonTree
}

func (p *BatchUpdateParameters) TreeDepth() uint32 {
	if len(p.MerkleProofs) == 0 {
		return 0
	}
	return uint32(len(p.MerkleProofs[0]))
}

func (p *BatchUpdateParameters) ValidateShape() error {
	if len(p.Leaves) != int(p.BatchSize) {
		return fmt.Errorf("wrong number of leaves: %d, expected: %d", len(p.Leaves), p.BatchSize)
	}
	if len(p.OldLeaves) != int(p.BatchSize) {
		return fmt.Errorf("wrong number of old leaves: %d, expected: %d", len(p.OldLeaves), p.BatchSize)
	}
	if len(p.TxHashes) != int(p.BatchSize) {
		return fmt.Errorf("wrong number of tx hashes: %d, expected: %d", len(p.TxHashes), p.BatchSize)
	}
	if len(p.MerkleProofs) != int(p.BatchSize) {
		return fmt.Errorf("wrong number of merkle proofs: %d", len(p.MerkleProofs))
	}
	if len(p.PathIndices) != int(p.BatchSize) {
		return fmt.Errorf("wrong number of path indices: %d", len(p.PathIndices))
	}
	for i, proof := range p.MerkleProofs {
		if len(proof) != int(p.Height) {
			return fmt.Errorf("wrong size of merkle proof for proof %d: %d", i, len(proof))
		}
	}
	return nil
}

func SetupBatchUpdate(height uint32, batchSize uint32) (*common.BatchProofSystem, error) {
	fmt.Println("Setting up batch update")
	ccs, err := R1CSBatchUpdate(height, batchSize)
	if err != nil {
		return nil, err
	}
	pk, vk, err := groth16.Setup(ccs)
	if err != nil {
		return nil, err
	}
	return &common.BatchProofSystem{
		CircuitType:      common.BatchUpdateCircuitType,
		TreeHeight:       height,
		BatchSize:        batchSize,
		ProvingKey:       pk,
		VerifyingKey:     vk,
		ConstraintSystem: ccs}, nil
}

func ProveBatchUpdate(ps *common.BatchProofSystem, params *BatchUpdateParameters) (*common.Proof, error) {
	if err := params.ValidateShape(); err != nil {
		return nil, err
	}

	publicInputHash := frontend.Variable(params.PublicInputHash)
	oldRoot := frontend.Variable(params.OldRoot)
	newRoot := frontend.Variable(params.NewRoot)
	leavesHashchainHash := frontend.Variable(params.LeavesHashchainHash)

	txHashes := make([]frontend.Variable, len(params.TxHashes))
	leaves := make([]frontend.Variable, len(params.Leaves))
	oldLeaves := make([]frontend.Variable, len(params.OldLeaves))
	pathIndices := make([]frontend.Variable, len(params.PathIndices))
	merkleProofs := make([][]frontend.Variable, len(params.MerkleProofs))

	for i := 0; i < len(params.Leaves); i++ {
		leaves[i] = frontend.Variable(params.Leaves[i])
		oldLeaves[i] = frontend.Variable(params.OldLeaves[i])
		txHashes[i] = frontend.Variable(params.TxHashes[i])
		pathIndices[i] = frontend.Variable(params.PathIndices[i])
		merkleProofs[i] = make([]frontend.Variable, len(params.MerkleProofs[i]))
		for j := 0; j < len(params.MerkleProofs[i]); j++ {
			merkleProofs[i][j] = frontend.Variable(params.MerkleProofs[i][j])
		}
	}

	assignment := BatchUpdateCircuit{
		PublicInputHash:     publicInputHash,
		OldRoot:             oldRoot,
		NewRoot:             newRoot,
		TxHashes:            txHashes,
		LeavesHashchainHash: leavesHashchainHash,
		OldLeaves:           oldLeaves,
		Leaves:              leaves,
		PathIndices:         pathIndices,
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

func R1CSBatchUpdate(height uint32, batchSize uint32) (constraint.ConstraintSystem, error) {
	leaves := make([]frontend.Variable, batchSize)
	oldLeaves := make([]frontend.Variable, batchSize)
	txHashes := make([]frontend.Variable, batchSize)
	pathIndices := make([]frontend.Variable, batchSize)
	merkleProofs := make([][]frontend.Variable, batchSize)

	for i := 0; i < int(batchSize); i++ {
		merkleProofs[i] = make([]frontend.Variable, height)
	}

	circuit := BatchUpdateCircuit{
		PublicInputHash:     frontend.Variable(0),
		OldRoot:             frontend.Variable(0),
		NewRoot:             frontend.Variable(0),
		TxHashes:            txHashes,
		LeavesHashchainHash: frontend.Variable(0),
		Leaves:              leaves,
		OldLeaves:           oldLeaves,
		PathIndices:         pathIndices,
		MerkleProofs:        merkleProofs,
		Height:              height,
		BatchSize:           batchSize,
	}

	return frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &circuit)
}

func ImportBatchUpdateSetup(treeHeight uint32, batchSize uint32, pkPath string, vkPath string) (*common.BatchProofSystem, error) {
	leaves := make([]frontend.Variable, batchSize)
	txHashes := make([]frontend.Variable, batchSize)
	oldLeaves := make([]frontend.Variable, batchSize)
	newMerkleProofs := make([][]frontend.Variable, batchSize)

	for i := 0; i < int(batchSize); i++ {
		newMerkleProofs[i] = make([]frontend.Variable, treeHeight)
	}

	circuit := BatchUpdateCircuit{
		Height:              treeHeight,
		TxHashes:            txHashes,
		Leaves:              leaves,
		OldLeaves:           oldLeaves,
		MerkleProofs:        newMerkleProofs,
		PathIndices:         make([]frontend.Variable, batchSize),
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
