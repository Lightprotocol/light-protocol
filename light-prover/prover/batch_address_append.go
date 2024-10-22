package prover

import (
	"fmt"
	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/constraint"
	"github.com/consensys/gnark/frontend/cs/r1cs"
	merkletree "light/light-prover/merkle-tree"
	"light/light-prover/prover/poseidon"
	"math/big"

	"github.com/consensys/gnark/frontend"
	"github.com/reilabs/gnark-lean-extractor/v2/abstractor"
)

// BatchAddressTreeAppendCircuit represents a zero-knowledge proof circuit for batch
// appending addresses to a Merkle tree.
type BatchAddressTreeAppendCircuit struct {
	PublicInputHash frontend.Variable `gnark:",public"`

	OldRoot       frontend.Variable `gnark:",private"`
	NewRoot       frontend.Variable `gnark:",private"`
	HashchainHash frontend.Variable `gnark:",private"`
	StartIndex    frontend.Variable `gnark:",private"`

	LowElementValues         []frontend.Variable   `gnark:",private"`
	OldLowElementNextIndices []frontend.Variable   `gnark:",private"`
	OldLowElementNextValues  []frontend.Variable   `gnark:",private"`
	LowElementNextIndices    []frontend.Variable   `gnark:",private"`
	LowElementNextValues     []frontend.Variable   `gnark:",private"`
	LowElementPathIndices    []frontend.Variable   `gnark:",private"`
	LowElementProofs         [][]frontend.Variable `gnark:",private"`

	NewElementValues      []frontend.Variable   `gnark:",private"`
	NewElementNextValues  []frontend.Variable   `gnark:",private"`
	NewElementNextIndices []frontend.Variable   `gnark:",private"`
	NewElementProofs      [][]frontend.Variable `gnark:",private"`

	BatchSize  uint32
	TreeHeight uint32
}

func (circuit *BatchAddressTreeAppendCircuit) Define(api frontend.API) error {
	var leafHashes []frontend.Variable
	currentRoot := circuit.OldRoot

	for i := uint32(0); i < circuit.BatchSize; i++ {
		abstractor.Call(api, LeafHashGadget{
			LeafLowerRangeValue:  circuit.LowElementValues[i],
			NextIndex:            circuit.OldLowElementNextIndices[i],
			LeafHigherRangeValue: circuit.OldLowElementNextValues[i],
			Value:                circuit.NewElementValues[i],
		})

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

		currentRoot = abstractor.Call(api, MerkleRootUpdateGadget{
			OldRoot:     currentRoot,
			OldLeaf:     oldLowLeafHash,
			NewLeaf:     lowLeafHash,
			PathIndex:   circuit.LowElementPathIndices[i],
			MerkleProof: circuit.LowElementProofs[i],
			Height:      int(circuit.TreeHeight),
		})

		currentRoot = abstractor.Call(api, MerkleRootUpdateGadget{
			OldRoot:     currentRoot,
			OldLeaf:     getZeroValue(0),
			NewLeaf:     newLeafHash,
			PathIndex:   circuit.LowElementNextIndices[i],
			MerkleProof: circuit.NewElementProofs[i],
			Height:      int(circuit.TreeHeight),
		})
	}

	api.AssertIsEqual(currentRoot, circuit.NewRoot)

	leavesHashChain := createHashChain(api, len(leafHashes), leafHashes)
	api.AssertIsEqual(leavesHashChain, circuit.HashchainHash)

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

	OldLowElements []merkletree.IndexedElement
	LowElements    []merkletree.IndexedElement
	NewElements    []merkletree.IndexedElement

	LowElementProofs [][]big.Int
	NewElementProofs [][]big.Int

	TreeHeight uint32
	BatchSize  uint32
	Tree       *merkletree.IndexedMerkleTree
}

func (p *BatchAddressTreeAppendParameters) ValidateShape() error {
	if len(p.OldLowElements) != int(p.BatchSize) {
		return fmt.Errorf("wrong nulber of old low elements: %d, expected: %d", len(p.OldLowElements), p.BatchSize)
	}
	if len(p.LowElements) != int(p.BatchSize) {
		return fmt.Errorf("wrong number of low elements: %d, expected: %d", len(p.LowElements), p.BatchSize)
	}
	if len(p.NewElements) != int(p.BatchSize) {
		return fmt.Errorf("wrong number of new elements: %d, expected: %d", len(p.NewElements), p.BatchSize)
	}
	if len(p.LowElementProofs) != int(p.BatchSize) {
		return fmt.Errorf("wrong number of low element proofs: %d, expected: %d", len(p.LowElementProofs), p.BatchSize)
	}
	if len(p.NewElementProofs) != int(p.BatchSize) {
		return fmt.Errorf("wrong number of new element proofs: %d, expected: %d", len(p.NewElementProofs), p.BatchSize)
	}
	return nil
}

func createAddressCircuit(height uint32, batchSize uint32) *BatchAddressTreeAppendCircuit {
	lowElementProofs := make([][]frontend.Variable, batchSize)
	newElementProofs := make([][]frontend.Variable, batchSize)

	for i := 0; i < int(batchSize); i++ {
		lowElementProofs[i] = make([]frontend.Variable, height)
		newElementProofs[i] = make([]frontend.Variable, height)
	}

	circuit := &BatchAddressTreeAppendCircuit{
		PublicInputHash: frontend.Variable(0),
		OldRoot:         frontend.Variable(0),
		NewRoot:         frontend.Variable(0),
		HashchainHash:   frontend.Variable(0),
		StartIndex:      frontend.Variable(0),

		OldLowElementNextIndices: make([]frontend.Variable, batchSize),
		OldLowElementNextValues:  make([]frontend.Variable, batchSize),

		LowElementValues:      make([]frontend.Variable, batchSize),
		LowElementNextValues:  make([]frontend.Variable, batchSize),
		LowElementNextIndices: make([]frontend.Variable, batchSize),
		LowElementPathIndices: make([]frontend.Variable, batchSize),
		LowElementProofs:      lowElementProofs,

		NewElementValues:      make([]frontend.Variable, batchSize),
		NewElementNextValues:  make([]frontend.Variable, batchSize),
		NewElementNextIndices: make([]frontend.Variable, batchSize),
		NewElementProofs:      newElementProofs,

		BatchSize:  batchSize,
		TreeHeight: height,
	}

	return circuit
}

func createAddressWitness(params *BatchAddressTreeAppendParameters) *BatchAddressTreeAppendCircuit {
	witness := createAddressCircuit(params.TreeHeight, params.BatchSize)

	witness.PublicInputHash = frontend.Variable(params.PublicInputHash)
	witness.OldRoot = params.OldRoot
	witness.NewRoot = params.NewRoot
	witness.HashchainHash = frontend.Variable(params.HashchainHash)
	witness.StartIndex = frontend.Variable(params.StartIndex)

	for i := uint32(0); i < params.BatchSize; i++ {
		witness.OldLowElementNextIndices[i] = frontend.Variable(params.OldLowElements[i].NextIndex)
		witness.OldLowElementNextValues[i] = frontend.Variable(params.OldLowElements[i].NextValue)

		witness.LowElementValues[i] = frontend.Variable(params.OldLowElements[i].Value)
		witness.LowElementNextValues[i] = frontend.Variable(params.LowElements[i].Value)
		witness.LowElementNextIndices[i] = frontend.Variable(params.LowElements[i].NextIndex)
		witness.LowElementPathIndices[i] = frontend.Variable(params.LowElements[i].Index)

		witness.NewElementValues[i] = frontend.Variable(params.NewElements[i].Value)
		witness.NewElementNextValues[i] = frontend.Variable(params.NewElements[i].NextValue)
		witness.NewElementNextIndices[i] = frontend.Variable(params.NewElements[i].NextIndex)

		witness.LowElementProofs[i] = make([]frontend.Variable, len(params.LowElementProofs[i]))
		witness.NewElementProofs[i] = make([]frontend.Variable, len(params.NewElementProofs[i]))

		for j := 0; j < len(params.LowElementProofs[i]); j++ {
			witness.LowElementProofs[i][j] = frontend.Variable(params.LowElementProofs[i][j])
		}
		for j := 0; j < len(params.NewElementProofs[i]); j++ {
			witness.NewElementProofs[i][j] = frontend.Variable(params.NewElementProofs[i][j])
		}

	}

	return witness
}

func R1CSBatchAddressAppend(height uint32, batchSize uint32) (constraint.ConstraintSystem, error) {
	circuit := createAddressCircuit(height, batchSize)
	return frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, circuit)
}

func ImportBatchAddressAppendSetup(height uint32, batchSize uint32, pkPath string, vkPath string) (*ProvingSystemV2, error) {
	ccs, err := R1CSBatchAddressAppend(height, batchSize)
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
		TreeHeight:       height,
		BatchSize:        batchSize,
		ProvingKey:       pk,
		VerifyingKey:     vk,
		ConstraintSystem: ccs,
	}, nil
}

func (ps *ProvingSystemV2) ProveBatchAddressAppend(params *BatchAddressTreeAppendParameters) (*Proof, error) {
	if err := params.ValidateShape(); err != nil {
		return nil, err
	}

	witnessCircuit := createAddressWitness(params)
	witness, err := frontend.NewWitness(witnessCircuit, ecc.BN254.ScalarField())
	proof, err := groth16.Prove(ps.ConstraintSystem, ps.ProvingKey, witness)
	if err != nil {
		return nil, fmt.Errorf("error proving: %v", err)
	}

	return &Proof{proof}, nil
}

func SetupBatchAddressAppend(height uint32, batchSize uint32) (*ProvingSystemV2, error) {
	fmt.Println("Setting up batch update")
	ccs, err := R1CSBatchAddressAppend(height, batchSize)
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
