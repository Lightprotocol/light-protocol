package prover

import (
	"fmt"
	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/constraint"
	"github.com/consensys/gnark/frontend"
	"github.com/reilabs/gnark-lean-extractor/v2/abstractor"
	"light/light-prover/logging"
	"light/light-prover/prover/poseidon"
	"os"
)

type Proof struct {
	Proof groth16.Proof
}

type ProvingSystem struct {
	TreeDepth        uint32
	NumberOfUtxos    uint32
	ProvingKey       groth16.ProvingKey
	VerifyingKey     groth16.VerifyingKey
	ConstraintSystem constraint.ConstraintSystem
}

// ProofRound gadget generates the ParentHash
type ProofRound struct {
	Direction frontend.Variable
	Hash      frontend.Variable
	Sibling   frontend.Variable
}

func (gadget ProofRound) DefineGadget(api frontend.API) interface{} {
	api.AssertIsBoolean(gadget.Direction)
	d1 := api.Select(gadget.Direction, gadget.Sibling, gadget.Hash)
	d2 := api.Select(gadget.Direction, gadget.Hash, gadget.Sibling)
	sum := abstractor.Call(api, poseidon.Poseidon2{In1: d1, In2: d2})
	return sum
}

// VerifyProof recovers the Merkle Tree using Proof[] and Path[] and returns the tree Root
// Proof[0] corresponds to the Leaf which is why len(Proof) === len(Path) + 1
type VerifyProof struct {
	Proof []frontend.Variable
	Path  []frontend.Variable
}

func (gadget VerifyProof) DefineGadget(api frontend.API) interface{} {
	sum := gadget.Proof[0]
	for i := 1; i < len(gadget.Proof); i++ {
		sum = abstractor.Call(api, ProofRound{Direction: gadget.Path[i-1], Hash: gadget.Proof[i], Sibling: sum})
	}
	return sum
}

type InclusionProof struct {
	Root           []frontend.Variable
	Leaf           []frontend.Variable
	InPathIndices  []frontend.Variable
	InPathElements [][]frontend.Variable

	NumberOfUtxos int
	Depth         int
}

func (gadget InclusionProof) DefineGadget(api frontend.API) interface{} {
	currentHash := make([]frontend.Variable, gadget.NumberOfUtxos)
	for proofIndex := 0; proofIndex < gadget.NumberOfUtxos; proofIndex++ {
		currentHash[proofIndex] = gadget.Leaf[proofIndex]
		for j := 0; j < gadget.Depth; j++ {
			currentHash[proofIndex] = abstractor.Call(api, ProofRound{Direction: gadget.InPathIndices[proofIndex], Hash: currentHash[proofIndex], Sibling: gadget.InPathElements[proofIndex][j]})
		}
	}
	return currentHash
}

// Trusted setup utility functions
// Taken from: https://github.com/bnb-chain/zkbnb/blob/master/common/prove/proof_keys.go#L19
func LoadProvingKey(filepath string) (pk groth16.ProvingKey, err error) {
	logging.Logger().Info().Msg("start reading proving key")
	pk = groth16.NewProvingKey(ecc.BN254)
	f, _ := os.Open(filepath)
	_, err = pk.ReadFrom(f)
	if err != nil {
		return pk, fmt.Errorf("read file error")
	}
	f.Close()

	return pk, nil
}

// Taken from: https://github.com/bnb-chain/zkbnb/blob/master/common/prove/proof_keys.go#L32
func LoadVerifyingKey(filepath string) (verifyingKey groth16.VerifyingKey, err error) {
	logging.Logger().Info().Msg("start reading verifying key")
	verifyingKey = groth16.NewVerifyingKey(ecc.BN254)
	f, _ := os.Open(filepath)
	_, err = verifyingKey.ReadFrom(f)
	if err != nil {
		return verifyingKey, fmt.Errorf("read file error")
	}
	f.Close()

	return verifyingKey, nil
}
