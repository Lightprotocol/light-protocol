package prover

import (
	"light/light-prover/prover/poseidon"
	"math/big"

	merkletree "light/light-prover/merkle-tree"

	"github.com/consensys/gnark/frontend"
	"github.com/reilabs/gnark-lean-extractor/v2/abstractor"
)

type BatchAddressTreeAppendCircuit struct {
	PublicInputHash frontend.Variable `gnark:",public"`

	OldRoot       frontend.Variable `gnark:",private"`
	NewRoot       frontend.Variable `gnark:",private"`
	HashchainHash frontend.Variable `gnark:",private"`
	StartIndex    frontend.Variable `gnark:",private"`

	LowElementValues      []frontend.Variable   `gnark:",private"`
	LowElementNextIndices []frontend.Variable   `gnark:",private"`
	LowElementNextValues  []frontend.Variable   `gnark:",private"`
	LowElementIndices     []frontend.Variable   `gnark:",private"`
	LowElementProofs      [][]frontend.Variable `gnark:",private"`

	NewElementValues []frontend.Variable   `gnark:",private"`
	NewElementProofs [][]frontend.Variable `gnark:",private"`

	BatchSize  uint32
	TreeHeight uint32
}

func (circuit *BatchAddressTreeAppendCircuit) Define(api frontend.API) error {
	currentRoot := circuit.OldRoot

	startIndexBits := api.ToBinary(circuit.StartIndex, int(circuit.TreeHeight))
	for i := uint32(0); i < circuit.BatchSize; i++ {
		oldLowLeafHash := abstractor.Call(api, LeafHashGadget{
			LeafLowerRangeValue:  circuit.LowElementValues[i],
			NextIndex:            circuit.LowElementNextIndices[i],
			LeafHigherRangeValue: circuit.LowElementNextValues[i],
			Value:                circuit.NewElementValues[i],
		})
		newLowLeafNextIndex := api.Add(circuit.StartIndex, i)

		lowLeafHash := abstractor.Call(api, poseidon.Poseidon3{
			In1: circuit.LowElementValues[i],
			In2: newLowLeafNextIndex,
			In3: circuit.NewElementValues[i],
		})
		pathIndexBits := api.ToBinary(circuit.LowElementIndices[i], int(circuit.TreeHeight))

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

	api.AssertIsEqual(circuit.NewRoot, currentRoot)

	leavesHashChain := createHashChain(api, int(circuit.BatchSize), circuit.NewElementValues)
	api.AssertIsEqual(circuit.HashchainHash, leavesHashChain)

	publicInputsHashChain := circuit.computePublicInputHash(api)
	api.AssertIsEqual(circuit.PublicInputHash, publicInputsHashChain)

	return nil
}

func (circuit *BatchAddressTreeAppendCircuit) computePublicInputHash(api frontend.API) frontend.Variable {
	hashChainInputs := []frontend.Variable{
		circuit.OldRoot,
		circuit.NewRoot,
		circuit.HashchainHash,
		circuit.StartIndex,
	}
	return createHashChain(api, 4, hashChainInputs)
}

type BatchAddressTreeAppendParameters struct {
	PublicInputHash *big.Int
	OldRoot         *big.Int
	NewRoot         *big.Int
	HashchainHash   *big.Int
	StartIndex      uint32

	LowElementValues      []big.Int
	LowElementIndices     []big.Int
	LowElementNextIndices []big.Int
	LowElementNextValues  []big.Int

	NewElementValues []big.Int

	LowElementProofs [][]big.Int
	NewElementProofs [][]big.Int

	TreeHeight uint32
	BatchSize  uint32
	Tree       *merkletree.IndexedMerkleTree
}
