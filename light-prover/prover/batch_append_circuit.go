package prover

import (
	"fmt"
	"light/light-prover/logging"
	merkle_tree "light/light-prover/merkle-tree"
	"math/big"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/constraint"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
	"github.com/reilabs/gnark-lean-extractor/v2/abstractor"
)

type BatchAppendCircuit struct {
	// Public inputs
	PublicInputHash     frontend.Variable `gnark:",public"`
	OldSubTreeHashChain frontend.Variable `gnark:",private"`
	NewSubTreeHashChain frontend.Variable `gnark:",private"`
	NewRoot             frontend.Variable `gnark:",private"`
	HashchainHash       frontend.Variable `gnark:",private"`
	StartIndex          frontend.Variable `gnark:",private"`

	// Private inputs
	Leaves   []frontend.Variable `gnark:",private"`
	Subtrees []frontend.Variable `gnark:",private"`

	BatchSize uint32
	// TODO: rename to height
	TreeHeight uint32
}

func (circuit *BatchAppendCircuit) Define(api frontend.API) error {
	if err := circuit.validateInputs(); err != nil {
		return err
	}
	hashChainInputs := make([]frontend.Variable, int(5))
	hashChainInputs[0] = circuit.OldSubTreeHashChain
	hashChainInputs[1] = circuit.NewSubTreeHashChain
	hashChainInputs[2] = circuit.NewRoot
	hashChainInputs[3] = circuit.HashchainHash
	hashChainInputs[4] = circuit.StartIndex

	publicInputsHashChain := createHashChain(api, int(5), hashChainInputs)

	api.AssertIsEqual(circuit.PublicInputHash, publicInputsHashChain)

	oldSubtreesHashChain := createHashChain(api, int(circuit.TreeHeight), circuit.Subtrees)
	api.AssertIsEqual(oldSubtreesHashChain, circuit.OldSubTreeHashChain)

	leavesHashChain := createHashChain(api, int(circuit.BatchSize), circuit.Leaves)
	api.AssertIsEqual(leavesHashChain, circuit.HashchainHash)

	newRoot, newSubtrees := circuit.batchAppend(api)
	api.AssertIsEqual(newRoot, circuit.NewRoot)

	newSubtreesHashChain := createHashChain(api, int(circuit.TreeHeight), newSubtrees)
	api.AssertIsEqual(newSubtreesHashChain, circuit.NewSubTreeHashChain)

	return nil
}

func (circuit *BatchAppendCircuit) validateInputs() error {
	if len(circuit.Leaves) != int(circuit.BatchSize) {
		return fmt.Errorf("leaves length (%d) does not match batch size (%d)", len(circuit.Leaves), circuit.BatchSize)
	}

	if len(circuit.Subtrees) != int(circuit.TreeHeight) {
		return fmt.Errorf("subtrees length (%d) does not match depth (%d)", len(circuit.Subtrees), circuit.TreeHeight)
	}

	return nil
}

func (circuit *BatchAppendCircuit) batchAppend(api frontend.API) (frontend.Variable, []frontend.Variable) {
	currentSubtrees := make([]frontend.Variable, len(circuit.Subtrees))
	copy(currentSubtrees, circuit.Subtrees)

	// Convert StartIndex to binary representation for tree traversal
	indexBits := api.ToBinary(circuit.StartIndex, int(circuit.TreeHeight))
	newRoot := frontend.Variable(0)

	for i := 0; i < int(circuit.BatchSize); i++ {
		leaf := circuit.Leaves[i]
		newRoot, currentSubtrees = circuit.append(api, leaf, currentSubtrees, indexBits)

		// Increment the binary representation of the index
		indexBits = incrementBits(api, indexBits)
	}

	return newRoot, currentSubtrees
}

// append inserts a new leaf into the Merkle tree and updates the tree structure accordingly.
// It traverses the tree from the bottom up, updating nodes and subtrees based on the binary
// representation of the insertion index.
//
// The function works as follows:
//  1. It starts with the new leaf as the current node.
//  2. For each level of the tree, from bottom to top:
//     a. It uses the corresponding bit of the index to determine if we're inserting on the right or left.
//     b. update subtrees:
//     - if isRight then do not update subtrees
//     - else update subtrees with current node
//     c. If inserting on the right (isRight is true):
//     - The current subtree at this level becomes the left sibling.
//     - The current node becomes the right sibling and the new subtree for this level.
//     d. If inserting on the left (isRight is false):
//     - The current node becomes the left sibling.
//     -  zero value becomes the right sibling.
//     e. It selects the appropriate left and right nodes based on the insertion direction.
//     f. It hashes the left and right nodes together to create the new parent node.
//     g. This new parent becomes the current node for the next level up.
//  3. The process repeats for each level, ultimately resulting in a new root hash.
//
// Parameters:
// - api: The frontend API for ZKP operations.
// - leaf: The new leaf to be inserted.
// - subtrees: The current state of subtrees in the Merkle tree.
// - indexBits: Binary representation of the insertion index.
//
// Returns:
// - The new root hash of the Merkle tree.
// - The updated subtrees after insertion.
func (circuit *BatchAppendCircuit) append(api frontend.API, leaf frontend.Variable, subtrees []frontend.Variable, indexBits []frontend.Variable) (frontend.Variable, []frontend.Variable) {
	currentNode := leaf

	for i := 0; i < int(circuit.TreeHeight); i++ {
		isRight := indexBits[i]
		subtrees[i] = api.Select(isRight, subtrees[i], currentNode)
		sibling := api.Select(isRight, subtrees[i], circuit.getZeroValue(api, i))

		currentNode = abstractor.Call(api, MerkleRootGadget{
			Hash:   currentNode,
			Index:  []frontend.Variable{isRight},
			Path:   []frontend.Variable{sibling},
			Height: 1,
		})
	}

	return currentNode, subtrees
}

// incrementBits implements binary addition to increment a number represented as a list of bits.
// It uses XOR and AND operations to efficiently increment the binary number without using
// traditional arithmetic operations, which is beneficial in zero-knowledge proof circuits.
//
// The function works as follows:
//  1. It starts with a carry of 1 (equivalent to adding 1 to the number).
//  2. For each bit, from least to most significant:
//     a. It XORs the current bit with the carry. This effectively adds the bit and carry
//     without considering a new carry. (0⊕0=0, 0⊕1=1, 1⊕0=1, 1⊕1=0)
//     b. It ANDs the original bit with the carry to determine if there should be a carry
//     for the next bit. (0∧0=0, 0∧1=0, 1∧0=0, 1∧1=1)
//     c. The result of XOR becomes the new value for the current bit.
//     d. The result of AND becomes the new carry for the next iteration.
//  3. This process continues for all bits, resulting in the incremented binary number.
//
// Example: Incrementing 0111 (7 in decimal)
// Initial state: 0111, carry = 1
// i=0: 1⊕1=0, carry=1∧1=1 -> 0110, carry=1
// i=1: 1⊕1=0, carry=1∧1=1 -> 0010, carry=1
// i=2: 1⊕1=0, carry=1∧1=1 -> 1010, carry=1
// i=3: 0⊕1=1, carry=0∧1=0 -> 1000, carry=0
// Final result: 1000 (8 in decimal)
func incrementBits(api frontend.API, bits []frontend.Variable) []frontend.Variable {
	carry := frontend.Variable(1)
	for i := 0; i < len(bits); i++ {
		// XOR operation implements binary addition without carry
		newBit := api.Xor(bits[i], carry)
		// AND operation determines if we need to carry to the next bit
		carry = api.And(bits[i], carry)
		bits[i] = newBit
	}
	return bits
}

func (circuit *BatchAppendCircuit) getZeroValue(api frontend.API, level int) frontend.Variable {
	return frontend.Variable(new(big.Int).SetBytes(ZERO_BYTES[level][:]))
}

type BatchAppendParameters struct {
	PublicInputHash     *big.Int   `json:"publicInputHash"`
	OldSubTreeHashChain *big.Int   `json:"oldSubTreeHashChain"`
	NewSubTreeHashChain *big.Int   `json:"newSubTreeHashChain"`
	NewRoot             *big.Int   `json:"newRoot"`
	HashchainHash       *big.Int   `json:"hashchainHash"`
	StartIndex          uint32     `json:"startIndex"`
	Leaves              []*big.Int `json:"leaves"`
	Subtrees            []*big.Int `json:"subtrees"`
	TreeHeight          uint32     `json:"treeHeight"`
	tree                *merkle_tree.PoseidonTree
}

func (p *BatchAppendParameters) BatchSize() uint32 {
	return uint32(len(p.Leaves))
}

func (p *BatchAppendParameters) ValidateShape(treeHeight uint32, batchSize uint32) error {
	if p.TreeHeight != treeHeight {
		return fmt.Errorf("wrong tree height: expected %d, got %d", treeHeight, p.TreeHeight)
	}
	if p.BatchSize() != batchSize {
		return fmt.Errorf("wrong batch size: expected %d, got %d", batchSize, p.BatchSize())
	}
	return nil
}

func SetupBatchAppend(treeHeight uint32, batchSize uint32) (*ProvingSystemV2, error) {
	ccs, err := R1CSBatchAppend(treeHeight, batchSize)

	if err != nil {
		return nil, err
	}

	pk, vk, err := groth16.Setup(ccs)
	if err != nil {
		return nil, err
	}

	return &ProvingSystemV2{
		TreeHeight:       treeHeight,
		BatchSize:        batchSize,
		ProvingKey:       pk,
		VerifyingKey:     vk,
		ConstraintSystem: ccs}, nil
}

func ImportBatchAppendSetup(treeDepth uint32, batchSize uint32, pkPath string, vkPath string) (*ProvingSystemV2, error) {
	circuit := BatchAppendCircuit{
		PublicInputHash:     frontend.Variable(0),
		OldSubTreeHashChain: frontend.Variable(0),
		NewSubTreeHashChain: frontend.Variable(0),
		NewRoot:             frontend.Variable(0),
		HashchainHash:       frontend.Variable(0),
		StartIndex:          frontend.Variable(0),
		Leaves:              make([]frontend.Variable, batchSize),
		Subtrees:            make([]frontend.Variable, treeDepth),
	}
	ccs, err := frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &circuit)
	if err != nil {
		return nil, fmt.Errorf("error compiling circuit: %v", err)
	}

	pk, err := LoadProvingKey(pkPath)
	if err != nil {
		return nil, fmt.Errorf("error loading proving key: %v", err)
	}

	vk, err := LoadVerifyingKey(vkPath)
	if err != nil {
		return nil, fmt.Errorf("error loading verifying key: %v", err)
	}
	return &ProvingSystemV2{
		TreeHeight:       treeDepth,
		BatchSize:        batchSize,
		ProvingKey:       pk,
		VerifyingKey:     vk,
		ConstraintSystem: ccs,
	}, nil
}

func (ps *ProvingSystemV2) ProveBatchAppend(params *BatchAppendParameters) (*Proof, error) {
	logging.Logger().Info().Msg("Starting Batch Append proof generation")
	logging.Logger().Info().Msg("Validating parameters")

	if err := params.ValidateShape(ps.TreeHeight, ps.BatchSize); err != nil {
		return nil, err
	}

	circuit := BatchAppendCircuit{
		PublicInputHash:     frontend.Variable(params.PublicInputHash),
		OldSubTreeHashChain: frontend.Variable(params.OldSubTreeHashChain),
		NewSubTreeHashChain: frontend.Variable(params.NewSubTreeHashChain),
		NewRoot:             frontend.Variable(params.NewRoot),
		HashchainHash:       frontend.Variable(params.HashchainHash),
		StartIndex:          frontend.Variable(params.StartIndex),
		Leaves:              make([]frontend.Variable, ps.BatchSize),
		Subtrees:            make([]frontend.Variable, ps.TreeHeight),
	}

	for i, leaf := range params.Leaves {
		circuit.Leaves[i] = frontend.Variable(leaf)
	}
	for i, subtree := range params.Subtrees {
		circuit.Subtrees[i] = frontend.Variable(subtree)
	}

	witness, err := frontend.NewWitness(&circuit, ecc.BN254.ScalarField())
	if err != nil {
		return nil, fmt.Errorf("error creating witness: %v", err)
	}

	logging.Logger().Info().Msg("Generating Batch Append proof")
	proof, err := groth16.Prove(ps.ConstraintSystem, ps.ProvingKey, witness)
	if err != nil {
		return nil, fmt.Errorf("error generating proof: %v", err)
	}

	logging.Logger().Info().Msg("Batch Append proof generated successfully")
	return &Proof{Proof: proof}, nil
}

func (ps *ProvingSystemV2) VerifyBatchAppend(oldSubTreeHashChain, newSubTreeHashChain, newRoot, hashchainHash *big.Int, proof *Proof) error {
	publicWitness := BatchAppendCircuit{
		OldSubTreeHashChain: frontend.Variable(oldSubTreeHashChain),
		NewSubTreeHashChain: frontend.Variable(newSubTreeHashChain),
		NewRoot:             frontend.Variable(newRoot),
		HashchainHash:       frontend.Variable(hashchainHash),
	}

	witness, err := frontend.NewWitness(&publicWitness, ecc.BN254.ScalarField(), frontend.PublicOnly())
	if err != nil {
		return fmt.Errorf("error creating public witness: %v", err)
	}

	err = groth16.Verify(proof.Proof, ps.VerifyingKey, witness)
	if err != nil {
		return fmt.Errorf("batch append proof verification failed: %v", err)
	}

	return nil
}

func R1CSBatchAppend(treeDepth uint32, batchSize uint32) (constraint.ConstraintSystem, error) {
	circuit := BatchAppendCircuit{
		PublicInputHash:     frontend.Variable(0),
		OldSubTreeHashChain: frontend.Variable(0),
		NewSubTreeHashChain: frontend.Variable(0),
		NewRoot:             frontend.Variable(0),
		HashchainHash:       frontend.Variable(0),
		StartIndex:          frontend.Variable(0),
		Leaves:              make([]frontend.Variable, batchSize),
		Subtrees:            make([]frontend.Variable, treeDepth),

		TreeHeight: treeDepth,
		BatchSize:  batchSize,
	}

	for i := range circuit.Leaves {
		circuit.Leaves[i] = frontend.Variable(0)
	}
	for i := range circuit.Subtrees {
		circuit.Subtrees[i] = frontend.Variable(0)
	}

	ccs, err := frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &circuit)
	if err != nil {
		return nil, err
	}

	return ccs, nil
}
