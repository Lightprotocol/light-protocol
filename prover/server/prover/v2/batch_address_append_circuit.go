package v2

import (
	"fmt"
	"light/light-prover/logging"
	"light/light-prover/prover/common"
	"light/light-prover/prover/poseidon"
	"math/big"

	merkletree "light/light-prover/merkle-tree"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/constraint"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
	"github.com/reilabs/gnark-lean-extractor/v3/abstractor"
)

type BatchAddressTreeAppendCircuit struct {
	PublicInputHash frontend.Variable `gnark:",public"`

	OldRoot       frontend.Variable `gnark:",secret"`
	NewRoot       frontend.Variable `gnark:",secret"`
	HashchainHash frontend.Variable `gnark:",secret"`
	StartIndex    frontend.Variable `gnark:",secret"`

	LowElementValues     []frontend.Variable   `gnark:",secret"`
	LowElementNextValues []frontend.Variable   `gnark:",secret"`
	LowElementIndices    []frontend.Variable   `gnark:",secret"`
	LowElementProofs     [][]frontend.Variable `gnark:",secret"`

	NewElementValues []frontend.Variable   `gnark:",secret"`
	NewElementProofs [][]frontend.Variable `gnark:",secret"`
	BatchSize        uint32
	TreeHeight       uint32
}

func (circuit *BatchAddressTreeAppendCircuit) Define(api frontend.API) error {
	currentRoot := circuit.OldRoot

	for i := uint32(0); i < circuit.BatchSize; i++ {
		oldLowLeafHash := abstractor.Call(api, common.LeafHashGadget{
			LeafLowerRangeValue:  circuit.LowElementValues[i],
			LeafHigherRangeValue: circuit.LowElementNextValues[i],
			Value:                circuit.NewElementValues[i],
		})

		lowLeafHash := abstractor.Call(api, poseidon.Poseidon2{
			In1: circuit.LowElementValues[i],
			In2: circuit.NewElementValues[i],
		})

		pathIndexBits := api.ToBinary(circuit.LowElementIndices[i], int(circuit.TreeHeight))
		currentRoot = abstractor.Call(api, common.MerkleRootUpdateGadget{
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
		newLeafHash := abstractor.Call(api, poseidon.Poseidon2{
			In1: circuit.NewElementValues[i],
			In2: circuit.LowElementNextValues[i],
		})

		indexBits := api.ToBinary(api.Add(circuit.StartIndex, i), int(circuit.TreeHeight))
		currentRoot = abstractor.Call(api, common.MerkleRootUpdateGadget{
			OldRoot:     currentRoot,
			OldLeaf:     getZeroValue(0),
			NewLeaf:     newLeafHash,
			PathIndex:   indexBits,
			MerkleProof: circuit.NewElementProofs[i],
			Height:      int(circuit.TreeHeight),
		})
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

	return createHashChain(api, hashChainInputs)
}

func InitBatchAddressTreeAppendCircuit(treeHeight uint32, batchSize uint32) BatchAddressTreeAppendCircuit {
	logging.Logger().Info().
		Uint32("treeHeight", treeHeight).
		Uint32("batchSize", batchSize).
		Msg("Initializing batch address append circuit")

	lowElementValues := make([]frontend.Variable, batchSize)
	lowElementNextValues := make([]frontend.Variable, batchSize)
	lowElementIndices := make([]frontend.Variable, batchSize)
	lowElementProofs := make([][]frontend.Variable, batchSize)
	newElementValues := make([]frontend.Variable, batchSize)
	newElementProofs := make([][]frontend.Variable, batchSize)

	for i := uint32(0); i < batchSize; i++ {
		lowElementProofs[i] = make([]frontend.Variable, treeHeight)
		newElementProofs[i] = make([]frontend.Variable, treeHeight)
	}

	return BatchAddressTreeAppendCircuit{
		BatchSize:            batchSize,
		TreeHeight:           treeHeight,
		PublicInputHash:      frontend.Variable(0),
		OldRoot:              frontend.Variable(0),
		NewRoot:              frontend.Variable(0),
		HashchainHash:        frontend.Variable(0),
		StartIndex:           frontend.Variable(0),
		LowElementValues:     lowElementValues,
		LowElementNextValues: lowElementNextValues,
		LowElementIndices:    lowElementIndices,
		LowElementProofs:     lowElementProofs,
		NewElementValues:     newElementValues,
		NewElementProofs:     newElementProofs,
	}
}

func (params *BatchAddressAppendParameters) CreateWitness() (*BatchAddressTreeAppendCircuit, error) {
	if params.BatchSize == 0 {
		return nil, fmt.Errorf("batch size cannot be 0")
	}
	if params.TreeHeight == 0 {
		return nil, fmt.Errorf("tree height cannot be 0")
	}

	circuit := &BatchAddressTreeAppendCircuit{
		BatchSize:            params.BatchSize,
		TreeHeight:           params.TreeHeight,
		PublicInputHash:      frontend.Variable(params.PublicInputHash),
		OldRoot:              frontend.Variable(params.OldRoot),
		NewRoot:              frontend.Variable(params.NewRoot),
		HashchainHash:        frontend.Variable(params.HashchainHash),
		StartIndex:           frontend.Variable(params.StartIndex),
		LowElementValues:     make([]frontend.Variable, params.BatchSize),
		LowElementNextValues: make([]frontend.Variable, params.BatchSize),
		LowElementIndices:    make([]frontend.Variable, params.BatchSize),
		NewElementValues:     make([]frontend.Variable, params.BatchSize),
		LowElementProofs:     make([][]frontend.Variable, params.BatchSize),
		NewElementProofs:     make([][]frontend.Variable, params.BatchSize),
	}

	for i := uint32(0); i < params.BatchSize; i++ {
		circuit.LowElementProofs[i] = make([]frontend.Variable, params.TreeHeight)
		circuit.NewElementProofs[i] = make([]frontend.Variable, params.TreeHeight)
	}

	for i := uint32(0); i < params.BatchSize; i++ {
		circuit.LowElementValues[i] = frontend.Variable(&params.LowElementValues[i])
		circuit.LowElementNextValues[i] = frontend.Variable(&params.LowElementNextValues[i])
		circuit.LowElementIndices[i] = frontend.Variable(&params.LowElementIndices[i])
		circuit.NewElementValues[i] = frontend.Variable(&params.NewElementValues[i])

		for j := uint32(0); j < params.TreeHeight; j++ {
			circuit.LowElementProofs[i][j] = frontend.Variable(&params.LowElementProofs[i][j])
			circuit.NewElementProofs[i][j] = frontend.Variable(&params.NewElementProofs[i][j])
		}
	}

	return circuit, nil
}
func (p *BatchAddressAppendParameters) ValidateShape() error {
	expectedArrayLen := int(p.BatchSize)
	expectedProofLen := int(p.TreeHeight)

	if len(p.LowElementValues) != expectedArrayLen {
		return fmt.Errorf("wrong number of low element values: %d, expected: %d",
			len(p.LowElementValues), expectedArrayLen)
	}
	if len(p.LowElementIndices) != expectedArrayLen {
		return fmt.Errorf("wrong number of low element indices: %d, expected: %d",
			len(p.LowElementIndices), expectedArrayLen)
	}
	if len(p.LowElementNextValues) != expectedArrayLen {
		return fmt.Errorf("wrong number of low element next values: %d, expected: %d",
			len(p.LowElementNextValues), expectedArrayLen)
	}
	if len(p.NewElementValues) != expectedArrayLen {
		return fmt.Errorf("wrong number of new element values: %d, expected: %d",
			len(p.NewElementValues), expectedArrayLen)
	}

	if len(p.LowElementProofs) != expectedArrayLen {
		return fmt.Errorf("wrong number of low element proofs: %d, expected: %d",
			len(p.LowElementProofs), expectedArrayLen)
	}
	if len(p.NewElementProofs) != expectedArrayLen {
		return fmt.Errorf("wrong number of new element proofs: %d, expected: %d",
			len(p.NewElementProofs), expectedArrayLen)
	}

	for i, proof := range p.LowElementProofs {
		if len(proof) != expectedProofLen {
			return fmt.Errorf("wrong proof length for LowElementProofs[%d]: got %d, expected %d",
				i, len(proof), expectedProofLen)
		}
	}
	for i, proof := range p.NewElementProofs {
		if len(proof) != expectedProofLen {
			return fmt.Errorf("wrong proof length for NewElementProofs[%d]: got %d, expected %d",
				i, len(proof), expectedProofLen)
		}
	}

	return nil
}

type BatchAddressAppendParameters struct {
	PublicInputHash *big.Int
	OldRoot         *big.Int
	NewRoot         *big.Int
	HashchainHash   *big.Int
	StartIndex      uint64

	LowElementValues     []big.Int
	LowElementIndices    []big.Int
	LowElementNextValues []big.Int

	NewElementValues []big.Int

	LowElementProofs [][]big.Int
	NewElementProofs [][]big.Int

	TreeHeight uint32
	BatchSize  uint32
	Tree       *merkletree.IndexedMerkleTree
}

func SetupBatchAddressAppend(height uint32, batchSize uint32) (*common.BatchProofSystem, error) {
	fmt.Println("Setting up address append batch update: height", height, "batch size", batchSize)
	ccs, err := R1CSBatchAddressAppend(height, batchSize)
	if err != nil {
		return nil, err
	}
	pk, vk, err := groth16.Setup(ccs)
	if err != nil {
		return nil, err
	}
	return &common.BatchProofSystem{
		CircuitType:      common.BatchAddressAppendCircuitType,
		TreeHeight:       height,
		BatchSize:        batchSize,
		ProvingKey:       pk,
		VerifyingKey:     vk,
		ConstraintSystem: ccs}, nil
}

func R1CSBatchAddressAppend(height uint32, batchSize uint32) (constraint.ConstraintSystem, error) {
	circuit := InitBatchAddressTreeAppendCircuit(height, batchSize)
	return frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &circuit)
}

func ProveBatchAddressAppend(ps *common.BatchProofSystem, params *BatchAddressAppendParameters) (*common.Proof, error) {
	if params == nil {
		panic("params cannot be nil")
	}

	if err := params.ValidateShape(); err != nil {
		return nil, err
	}

	assignment, err := params.CreateWitness()
	if err != nil {
		return nil, fmt.Errorf("error creating circuit: %v", err)
	}

	witness, err := frontend.NewWitness(assignment, ecc.BN254.ScalarField())
	if err != nil {
		return nil, fmt.Errorf("error creating witness: %v", err)
	}

	proof, err := groth16.Prove(ps.ConstraintSystem, ps.ProvingKey, witness)
	if err != nil {
		return nil, fmt.Errorf("error proving: %v", err)
	}

	return &common.Proof{proof}, nil
}

func ImportBatchAddressAppendSetup(treeHeight uint32, batchSize uint32, pkPath string, vkPath string) (*common.BatchProofSystem, error) {
	circuit := InitBatchAddressTreeAppendCircuit(batchSize, treeHeight)

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
		CircuitType:      common.BatchAddressAppendCircuitType,
		TreeHeight:       treeHeight,
		BatchSize:        batchSize,
		ProvingKey:       pk,
		VerifyingKey:     vk,
		ConstraintSystem: ccs,
	}, nil
}

func ImportBatchAddressAppendSetupWithR1CS(treeHeight uint32, batchSize uint32, pkPath string, vkPath string, r1csPath string) (*common.BatchProofSystem, error) {
	pk, err := common.LoadProvingKey(pkPath)
	if err != nil {
		return nil, err
	}

	vk, err := common.LoadVerifyingKey(vkPath)
	if err != nil {
		return nil, err
	}

	ccs, err := common.LoadConstraintSystem(r1csPath)
	if err != nil {
		return nil, err
	}

	return &common.BatchProofSystem{
		CircuitType:      common.BatchAddressAppendCircuitType,
		TreeHeight:       treeHeight,
		BatchSize:        batchSize,
		ProvingKey:       pk,
		VerifyingKey:     vk,
		ConstraintSystem: ccs,
	}, nil
}
