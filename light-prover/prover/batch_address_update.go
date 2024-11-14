package prover

import (
	merkletree "light/light-prover/merkle-tree"
	"light/light-prover/prover/poseidon"
	"math/big"

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
	LowElementValues         []frontend.Variable   `gnark:",private"`
	OldLowElementNextIndices []frontend.Variable   `gnark:",private"`
	OldLowElementNextValues  []frontend.Variable   `gnark:",private"`
	LowElementNextIndices    []frontend.Variable   `gnark:",private"`
	LowElementNextValues     []frontend.Variable   `gnark:",private"`
	LowElementPathIndices    []frontend.Variable   `gnark:",private"`
	LowElementProofs         [][]frontend.Variable `gnark:",private"`

	// New elements being inserted
	NewElementValues      []frontend.Variable   `gnark:",private"`
	NewElementNextValues  []frontend.Variable   `gnark:",private"`
	NewElementNextIndices []frontend.Variable   `gnark:",private"`
	NewElementProofs      [][]frontend.Variable `gnark:",private"`

	// Circuit configuration
	BatchSize  uint32
	TreeHeight uint32
}

// Define implements the circuit's constraints and verification logic
func (circuit *BatchAddressTreeAppendCircuit) Define(api frontend.API) error {
	var leafHashes []frontend.Variable
	currentRoot := circuit.OldRoot

	startIndexBits := api.ToBinary(circuit.StartIndex, int(circuit.TreeHeight))
	// Process each element in the batch
	for i := uint32(0); i < circuit.BatchSize; i++ {
		// Verify value ordering and proper linking between elements
		abstractor.Call(api, LeafHashGadget{
			LeafLowerRangeValue:  circuit.LowElementValues[i],
			NextIndex:            circuit.OldLowElementNextIndices[i],
			LeafHigherRangeValue: circuit.OldLowElementNextValues[i],
			Value:                circuit.NewElementValues[i],
		})

		// Calculate hashes for current batch element
		oldLowLeafHash := abstractor.Call(api, poseidon.Poseidon3{
			In1: circuit.LowElementValues[i],
			In2: circuit.OldLowElementNextIndices[i],
			In3: circuit.OldLowElementNextValues[i],
		})

		lowLeafHash := abstractor.Call(api, poseidon.Poseidon3{
			In1: circuit.LowElementNextValues[i],
			In2: circuit.LowElementNextIndices[i],
			In3: circuit.NewElementValues[i],
		})
		leafHashes = append(leafHashes, lowLeafHash)

		newLeafHash := abstractor.Call(api, poseidon.Poseidon3{
			In1: circuit.NewElementValues[i],
			In2: circuit.NewElementNextIndices[i],
			In3: circuit.NewElementNextValues[i],
		})
		leafHashes = append(leafHashes, newLeafHash)
		pathIndexBits := api.ToBinary(circuit.LowElementPathIndices[i], int(circuit.TreeHeight))

		// Update Merkle root for both low and new elements
		currentRoot = abstractor.Call(api, MerkleRootUpdateGadget{
			OldRoot:     currentRoot,
			OldLeaf:     oldLowLeafHash,
			NewLeaf:     lowLeafHash,
			PathIndex:   pathIndexBits,
			MerkleProof: circuit.LowElementProofs[i],
			Height:      int(circuit.TreeHeight),
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
	api.AssertIsEqual(currentRoot, circuit.NewRoot)

	// Calculate and verify leaf hash chain
	leavesHashChain := createHashChain(api, len(leafHashes), leafHashes)
	api.AssertIsEqual(leavesHashChain, circuit.HashchainHash)

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
	OldRoot         frontend.Variable
	NewRoot         frontend.Variable
	HashchainHash   *big.Int
	StartIndex      uint32

	// Elements being modified or added
	OldLowElements []merkletree.IndexedElement
	LowElements    []merkletree.IndexedElement
	NewElements    []merkletree.IndexedElement

	// Merkle proofs for verification
	LowElementProofs [][]big.Int
	NewElementProofs [][]big.Int

	// Tree configuration
	TreeHeight uint32
	BatchSize  uint32
	Tree       *merkletree.IndexedMerkleTree
}
