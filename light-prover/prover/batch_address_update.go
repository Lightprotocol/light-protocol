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
	/*
		before append:
		leaf 0 18107977475760319057966144103673937810858686565134338371146286848755066863726
		leaf 1 13859306649965657812382249699983066845935552967038026417581136538215435035637

		after append:
		leaf 0 11849662192759696951949663276223249598333033836968262700464268262562959838751
		leaf 1 13859306649965657812382249699983066845935552967038026417581136538215435035637
		leaf 2 10176911339282415347266153466195213091952072716480101797118796476634110903352
	*/
	currentRoot := circuit.OldRoot

	// print low element proofs
	for i := 0; i < int(circuit.BatchSize); i++ {
		api.Println("LowElementProofs[", i, "]: ", circuit.LowElementProofs[i])
	}

	startIndexBits := api.ToBinary(circuit.StartIndex, int(circuit.TreeHeight))
	for i := uint32(0); i < circuit.BatchSize; i++ {
		api.Println("LowElementValues[", i, "]: ", circuit.LowElementValues[i])
		api.Println("LowElementNextIndices[", i, "]: ", circuit.LowElementNextIndices[i])
		api.Println("LowElementNextValues[", i, "]: ", circuit.LowElementNextValues[i])
		api.Println("NewElementValues[", i, "]: ", circuit.NewElementValues[i])

		oldLowLeafHash := abstractor.Call(api, LeafHashGadget{
			LeafLowerRangeValue:  circuit.LowElementValues[i],
			NextIndex:            circuit.LowElementNextIndices[i],
			LeafHigherRangeValue: circuit.LowElementNextValues[i],
			Value:                circuit.NewElementValues[i],
		})
		api.Println("OldLowLeafHash: ", oldLowLeafHash)

		newLowLeafNextIndex := api.Add(circuit.StartIndex, i)
		api.Println("NewLowLeafNextIndex: ", newLowLeafNextIndex)

		lowLeafHash := abstractor.Call(api, poseidon.Poseidon3{
			In1: circuit.LowElementValues[i],
			In2: newLowLeafNextIndex,
			In3: circuit.NewElementValues[i],
		})
		api.Println("LowLeafHash: ", lowLeafHash)

		pathIndexBits := api.ToBinary(circuit.LowElementIndices[i], int(circuit.TreeHeight))
		api.Println("PathIndexBits: ", pathIndexBits)

		currentRoot = abstractor.Call(api, MerkleRootUpdateGadget{
			OldRoot:     currentRoot,
			OldLeaf:     oldLowLeafHash,
			NewLeaf:     lowLeafHash,
			PathIndex:   pathIndexBits,
			MerkleProof: circuit.LowElementProofs[i],
			Height:      int(circuit.TreeHeight),
		})
		api.Println("CurrentRoot: ", currentRoot)

		// value = new value
		// next value is low leaf next value
		// next index is new value next index
		newLeafHash := abstractor.Call(api, poseidon.Poseidon3{
			In1: circuit.NewElementValues[i],
			In2: circuit.LowElementNextIndices[i],
			In3: circuit.LowElementNextValues[i],
		})
		api.Println("NewLeafHash: ", newLeafHash)

		currentRoot = abstractor.Call(api, MerkleRootUpdateGadget{
			OldRoot:     currentRoot,
			OldLeaf:     getZeroValue(0),
			NewLeaf:     newLeafHash,
			PathIndex:   startIndexBits,
			MerkleProof: circuit.NewElementProofs[i],
			Height:      int(circuit.TreeHeight),
		})
		api.Println("CurrentRoot: ", currentRoot)

		startIndexBits = incrementBits(
			api,
			startIndexBits,
		)
		api.Println("StartIndexBits: ", startIndexBits)
	}

	api.AssertIsEqual(circuit.NewRoot, currentRoot)

	leavesHashChain := createHashChain(api, circuit.NewElementValues)
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
	api.Println("Computing public input hash with inputs: ", hashChainInputs)

	return createHashChain(api, hashChainInputs)
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
