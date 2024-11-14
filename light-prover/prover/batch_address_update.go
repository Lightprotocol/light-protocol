package prover

import (
	"light/light-prover/prover/poseidon"
	"math/big"

	merkletree "light/light-prover/merkle-tree"

	"github.com/consensys/gnark/frontend"
	"github.com/reilabs/gnark-lean-extractor/v2/abstractor"
)

// BatchAddressTreeAppendCircuit represents a zero-knowledge proof circuit for batch
// appending addresses to a Merkle tree.
type BatchAddressTreeAppendCircuit struct {
	// Public inputs that can be verified by anyone
	PublicInputHash frontend.Variable `gnark:",public"`

	// Private inputs for tree state
	OldRoot       frontend.Variable `gnark:",private"`
	NewRoot       frontend.Variable `gnark:",private"`
	HashchainHash frontend.Variable `gnark:",private"`
	StartIndex    frontend.Variable `gnark:",private"`

	// Element values and linking information
	LowElementValues      []frontend.Variable   `gnark:",private"`
	LowElementNextIndices []frontend.Variable   `gnark:",private"`
	LowElementNextValues  []frontend.Variable   `gnark:",private"`
	LowElementIndices     []frontend.Variable   `gnark:",private"`
	LowElementProofs      [][]frontend.Variable `gnark:",private"`

	// New elements being inserted
	NewElementValues []frontend.Variable   `gnark:",private"`
	NewElementProofs [][]frontend.Variable `gnark:",private"`

	// Circuit configuration
	BatchSize  uint32
	TreeHeight uint32
}

// Define implements the circuit's constraints and verification logic
func (circuit *BatchAddressTreeAppendCircuit) Define(api frontend.API) error {
	currentRoot := circuit.OldRoot

	startIndexBits := api.ToBinary(circuit.StartIndex, int(circuit.TreeHeight))
	// Process each element in the batch
	for i := uint32(0); i < circuit.BatchSize; i++ {
		// Verify value ordering and proper linking between elements
		oldLowLeafHash := abstractor.Call(api, LeafHashGadget{
			LeafLowerRangeValue:  circuit.LowElementValues[i],
			NextIndex:            circuit.LowElementNextIndices[i],
			LeafHigherRangeValue: circuit.LowElementNextValues[i],
			Value:                circuit.NewElementValues[i],
		})
		newLowLeafNextIndex := api.Add(circuit.StartIndex, i)
		// low leaf value stays the same
		// next index is new value index = newLowLeafNextIndex
		// next value is new value
		lowLeafHash := abstractor.Call(api, poseidon.Poseidon3{
			In1: circuit.LowElementValues[i],
			In2: newLowLeafNextIndex,
			In3: circuit.NewElementValues[i],
		})
		pathIndexBits := api.ToBinary(circuit.LowElementIndices[i], int(circuit.TreeHeight))

		// Update Merkle root for both low and new elements
		currentRoot = abstractor.Call(api, MerkleRootUpdateGadget{
			OldRoot:     currentRoot,
			OldLeaf:     oldLowLeafHash,
			NewLeaf:     lowLeafHash,
			PathIndex:   pathIndexBits,
			MerkleProof: circuit.LowElementProofs[i],
			Height:      int(circuit.TreeHeight),
		})
		// value = new value
		// next value is low leaf next value
		// next index is new value next index
		newLeafHash := abstractor.Call(api, poseidon.Poseidon3{
			In1: circuit.NewElementValues[i],
			In2: circuit.LowElementNextIndices[i],
			In3: circuit.LowElementNextValues[i],
		})

		currentRoot = abstractor.Call(api, MerkleRootUpdateGadget{
			OldRoot:     currentRoot,
			OldLeaf:     getZeroValue(0),
			NewLeaf:     newLeafHash,
			PathIndex:   startIndexBits,
			MerkleProof: circuit.NewElementProofs[i],
			Height:      int(circuit.TreeHeight),
		})
		startIndexBits = incrementBits(
			api,
			startIndexBits,
		)
	}

	// Verify the final root matches
	api.AssertIsEqual(circuit.NewRoot, currentRoot)

	// Calculate and verify leaf hash chain
	leavesHashChain := createHashChain(api, int(circuit.BatchSize), circuit.NewElementValues)
	api.AssertIsEqual(circuit.HashchainHash, leavesHashChain)

	// Verify public input hash
	publicInputsHashChain := circuit.computePublicInputHash(api)
	api.AssertIsEqual(circuit.PublicInputHash, publicInputsHashChain)

	return nil
}

// computePublicInputHash calculates the hash of all public inputs
func (circuit *BatchAddressTreeAppendCircuit) computePublicInputHash(api frontend.API) frontend.Variable {
	hashChainInputs := []frontend.Variable{
		circuit.OldRoot,
		circuit.NewRoot,
		circuit.HashchainHash,
		circuit.StartIndex,
	}
	return createHashChain(api, 4, hashChainInputs)
}

// BatchAddressTreeAppendParameters holds the parameters needed for batch address updates
type BatchAddressTreeAppendParameters struct {
	PublicInputHash *big.Int
	OldRoot         *big.Int
	NewRoot         *big.Int
	HashchainHash   *big.Int
	StartIndex      uint32

	// Elements being modified or added
	LowElementValues      []big.Int
	LowElementIndices     []big.Int
	LowElementNextIndices []big.Int
	LowElementNextValues  []big.Int

	NewElementValues []big.Int

	// Merkle proofs for verification
	LowElementProofs [][]big.Int
	NewElementProofs [][]big.Int

	// Tree configuration
	TreeHeight uint32
	BatchSize  uint32
	Tree       *merkletree.IndexedMerkleTree
}
