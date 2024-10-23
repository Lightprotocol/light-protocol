package prover

import (
	"fmt"
	merkletree "light/light-prover/merkle-tree"
	"light/light-prover/prover/poseidon"
	"math/big"

	"github.com/consensys/gnark/frontend"
	"github.com/reilabs/gnark-lean-extractor/v2/abstractor"
)

// Circuit Definition
type BatchAddressTreeAppendCircuit struct {
	// Public inputs
	PublicInputHash     frontend.Variable `gnark:",public"`
	OldSubTreeHashChain frontend.Variable `gnark:",private"`
	NewSubTreeHashChain frontend.Variable `gnark:",private"`
	NewRoot             frontend.Variable `gnark:",private"`
	HashchainHash       frontend.Variable `gnark:",private"`
	StartIndex          frontend.Variable `gnark:",private"`

	// Private inputs for non-inclusion proof
	LowElementValues      []frontend.Variable   `gnark:",private"`
	LowElementNextValues  []frontend.Variable   `gnark:",private"`
	LowElementNextIndices []frontend.Variable   `gnark:",private"`
	LowElementProofs      [][]frontend.Variable `gnark:",private"`
	LowElementPathIndices []frontend.Variable   `gnark:",private"`

	// Private inputs for batch append
	NewElementValues []frontend.Variable `gnark:",private"`
	Subtrees         []frontend.Variable `gnark:",private"`

	BatchSize  uint32
	TreeHeight uint32
}

// Circuit Define implementation
func (circuit *BatchAddressTreeAppendCircuit) Define(api frontend.API) error {
	if err := circuit.validateInputs(); err != nil {
		return err
	}

	// Create hash chain of public inputs
	hashChainInputs := make([]frontend.Variable, 5)
	hashChainInputs[0] = circuit.OldSubTreeHashChain
	hashChainInputs[1] = circuit.NewSubTreeHashChain
	hashChainInputs[2] = circuit.NewRoot
	hashChainInputs[3] = circuit.HashchainHash
	hashChainInputs[4] = circuit.StartIndex

	publicInputsHashChain := createHashChainCircuit(api, 5, hashChainInputs)
	api.AssertIsEqual(circuit.PublicInputHash, publicInputsHashChain)

	// 1. Process each low element and validate new elements
	currentRoot := circuit.NewRoot

	for i := uint32(0); i < circuit.BatchSize; i++ {
		// 1.1 Validate new element
		// Check that new element value is greater than low element value
		abstractor.CallVoid(api, AssertIsLess{
			A: circuit.LowElementValues[i],
			B: circuit.NewElementValues[i],
			N: 248,
		})

		// If next index is not zero, check that new element is less than next value
		isNextIndexZero := api.IsZero(circuit.LowElementNextIndices[i])

		// Create constraint when next index is not zero
		nextIndexCheck := abstractor.Call(api, AssertIsLess{
			A: circuit.NewElementValues[i],
			B: circuit.LowElementNextValues[i],
			N: 248,
		})

		// The constraint is only enforced when isNextIndexZero is 0 (false)
		api.Select(isNextIndexZero, 1, nextIndexCheck)

		// 1.2 Update low element
		// Hash the updated low element with poseidon
		newLowLeaf := abstractor.Call(api, poseidon.Poseidon3{
			In1: circuit.LowElementValues[i],
			In2: circuit.LowElementNextIndices[i],
			In3: circuit.NewElementValues[i],
		})

		// Verify merkle proof for low element
		oldRoot := abstractor.Call(api, MerkleRootGadget{
			Hash:   newLowLeaf,
			Index:  circuit.LowElementPathIndices[i],
			Path:   circuit.LowElementProofs[i],
			Height: int(circuit.TreeHeight),
		})
		api.AssertIsEqual(oldRoot, currentRoot)

		// Hash the new element
		newLeaf := abstractor.Call(api, poseidon.Poseidon3{
			In1: circuit.NewElementValues[i],
			In2: circuit.LowElementNextIndices[i],
			In3: circuit.LowElementNextValues[i],
		})

		// Update root
		currentRoot = abstractor.Call(api, MerkleRootGadget{
			Hash:   newLeaf,
			Index:  circuit.LowElementPathIndices[i],
			Path:   circuit.LowElementProofs[i],
			Height: int(circuit.TreeHeight),
		})
	}

	// 2. Batch append
	oldSubtreesHashChain := createHashChainCircuit(api, int(circuit.TreeHeight), circuit.Subtrees)
	api.AssertIsEqual(oldSubtreesHashChain, circuit.OldSubTreeHashChain)

	newLeaves := make([]frontend.Variable, circuit.BatchSize)
	for i := uint32(0); i < circuit.BatchSize; i++ {
		newLeaves[i] = abstractor.Call(api, poseidon.Poseidon3{
			In1: circuit.NewElementValues[i],
			In2: circuit.LowElementNextIndices[i],
			In3: circuit.LowElementNextValues[i],
		})
	}

	leavesHashChain := createHashChainCircuit(api, int(circuit.BatchSize), newLeaves)
	api.AssertIsEqual(leavesHashChain, circuit.HashchainHash)

	finalRoot, newSubtrees := circuit.batchAppend(api, newLeaves)
	api.AssertIsEqual(finalRoot, currentRoot)

	newSubtreesHashChain := createHashChainCircuit(api, int(circuit.TreeHeight), newSubtrees)
	api.AssertIsEqual(newSubtreesHashChain, circuit.NewSubTreeHashChain)

	return nil
}

// Circuit helper functions
func (circuit *BatchAddressTreeAppendCircuit) validateInputs() error {
	if len(circuit.NewElementValues) != int(circuit.BatchSize) {
		return fmt.Errorf("new elements length (%d) does not match batch size (%d)",
			len(circuit.NewElementValues), circuit.BatchSize)
	}
	if len(circuit.LowElementValues) != int(circuit.BatchSize) {
		return fmt.Errorf("low elements length (%d) does not match batch size (%d)",
			len(circuit.LowElementValues), circuit.BatchSize)
	}
	if len(circuit.Subtrees) != int(circuit.TreeHeight) {
		return fmt.Errorf("subtrees length (%d) does not match tree height (%d)",
			len(circuit.Subtrees), circuit.TreeHeight)
	}
	for i := 0; i < int(circuit.BatchSize); i++ {
		if len(circuit.LowElementProofs[i]) != int(circuit.TreeHeight) {
			return fmt.Errorf("merkle proof %d length (%d) does not match tree height (%d)",
				i, len(circuit.LowElementProofs[i]), circuit.TreeHeight)
		}
	}
	return nil
}

func createHashChainCircuit(api frontend.API, length int, inputs []frontend.Variable) frontend.Variable {
	if len(inputs) == 0 {
		return frontend.Variable(0)
	}
	if len(inputs) == 1 {
		return inputs[0]
	}

	hashChain := inputs[0]
	for i := 1; i < length; i++ {
		hashChain = abstractor.Call(api, poseidon.Poseidon2{
			In1: hashChain,
			In2: inputs[i],
		})
	}
	return hashChain
}

func (circuit *BatchAddressTreeAppendCircuit) batchAppend(
	api frontend.API,
	leaves []frontend.Variable,
) (frontend.Variable, []frontend.Variable) {
	currentSubtrees := make([]frontend.Variable, len(circuit.Subtrees))
	copy(currentSubtrees, circuit.Subtrees)

	indexBits := api.ToBinary(circuit.StartIndex, int(circuit.TreeHeight))
	newRoot := frontend.Variable(0)

	for i := 0; i < int(circuit.BatchSize); i++ {
		leaf := leaves[i]
		newRoot, currentSubtrees = circuit.append(api, leaf, currentSubtrees, indexBits)
		indexBits = circuit.incrementBits(api, indexBits)
	}

	return newRoot, currentSubtrees
}

func (circuit *BatchAddressTreeAppendCircuit) append(
	api frontend.API,
	leaf frontend.Variable,
	subtrees []frontend.Variable,
	indexBits []frontend.Variable,
) (frontend.Variable, []frontend.Variable) {
	currentNode := leaf
	for i := 0; i < int(circuit.TreeHeight); i++ {
		isRight := indexBits[i]
		subtrees[i] = api.Select(isRight, subtrees[i], currentNode)
		sibling := api.Select(isRight, subtrees[i], circuit.getZeroValue(api, i))

		currentNode = abstractor.Call(api, MerkleRootGadget{
			Hash:   currentNode,
			Index:  isRight,
			Path:   []frontend.Variable{sibling},
			Height: 1,
		})
	}
	return currentNode, subtrees
}

func (circuit *BatchAddressTreeAppendCircuit) incrementBits(
	api frontend.API,
	bits []frontend.Variable,
) []frontend.Variable {
	carry := frontend.Variable(1)
	for i := 0; i < len(bits); i++ {
		newBit := api.Xor(bits[i], carry)
		carry = api.And(bits[i], carry)
		bits[i] = newBit
	}
	return bits
}

func (circuit *BatchAddressTreeAppendCircuit) getZeroValue(api frontend.API, level int) frontend.Variable {
	return frontend.Variable(new(big.Int).SetBytes(ZERO_BYTES[level][:]))
}

// Parameters struct
type BatchAddressTreeAppendParameters struct {
	// Public inputs
	PublicInputHash     *big.Int
	OldSubTreeHashChain *big.Int
	NewSubTreeHashChain *big.Int
	NewRoot             *big.Int
	HashchainHash       *big.Int
	StartIndex          uint32

	// Low elements data
	LowElementValues      []*big.Int
	LowElementNextValues  []*big.Int
	LowElementNextIndices []uint32
	LowElementProofs      [][]big.Int
	LowElementPathIndices []uint32

	// New elements
	NewElementValues []*big.Int
	Subtrees         []*big.Int

	TreeHeight uint32
	BatchSize  uint32
	Tree       *merkletree.PoseidonTree
}
