package masp

import (
	"fmt"

	"light/light-prover/prover/common"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
)

// CompileUtxoSystem compiles UtxoCircuit at shape (nInputs, nOutputs) and
// runs an in-process Groth16 setup. The resulting proving/verifying keys are
// not stable across processes; production deployments must replace this with
// a key-registry-backed loader (see Phase 7.5).
func CompileUtxoSystem(nInputs, nOutputs uint32) (*common.MaspProofSystem, error) {
	if nInputs == 0 || nOutputs == 0 {
		return nil, fmt.Errorf("masp utxo: nInputs and nOutputs must be > 0")
	}
	circuit := NewUtxoCircuit(int(nInputs), int(nOutputs))
	cs, err := frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, circuit)
	if err != nil {
		return nil, fmt.Errorf("masp utxo compile: %w", err)
	}
	pk, vk, err := groth16.Setup(cs)
	if err != nil {
		return nil, fmt.Errorf("masp utxo setup: %w", err)
	}
	return &common.MaspProofSystem{
		CircuitType:        common.MaspUtxoCircuitType,
		NInputs:            nInputs,
		NOutputs:           nOutputs,
		StateTreeDepth:     StateTreeHeight,
		NullifierTreeDepth: IndexedTreeHeight,
		ProvingKey:         pk,
		VerifyingKey:       vk,
		ConstraintSystem:   cs,
	}, nil
}

// CompileTreeSystem compiles TreeCircuit at NInputs and runs setup.
func CompileTreeSystem(nInputs, _ uint32) (*common.MaspProofSystem, error) {
	if nInputs == 0 {
		return nil, fmt.Errorf("masp tree: nInputs must be > 0")
	}
	circuit := NewTreeCircuit(int(nInputs))
	cs, err := frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, circuit)
	if err != nil {
		return nil, fmt.Errorf("masp tree compile: %w", err)
	}
	pk, vk, err := groth16.Setup(cs)
	if err != nil {
		return nil, fmt.Errorf("masp tree setup: %w", err)
	}
	return &common.MaspProofSystem{
		CircuitType:        common.MaspTreeCircuitType,
		NInputs:            nInputs,
		NOutputs:           0,
		StateTreeDepth:     StateTreeHeight,
		NullifierTreeDepth: IndexedTreeHeight,
		ProvingKey:         pk,
		VerifyingKey:       vk,
		ConstraintSystem:   cs,
	}, nil
}

// RegisterDefaultBuilders attaches the in-process compile+setup builders to a
// LazyKeyManager so /prove can serve MASP requests in dev environments. The
// production server should call a key-registry-backed registration instead.
func RegisterDefaultBuilders(km *common.LazyKeyManager) {
	km.RegisterMaspBuilder(common.MaspUtxoCircuitType, CompileUtxoSystem)
	km.RegisterMaspBuilder(common.MaspTreeCircuitType, CompileTreeSystem)
}
