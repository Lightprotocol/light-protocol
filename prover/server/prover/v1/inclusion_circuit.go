package v1

import (
	"light/light-prover/prover/common"

	"github.com/consensys/gnark/frontend"
	"github.com/reilabs/gnark-lean-extractor/v3/abstractor"
)

type InclusionCircuit struct {
	// public inputs
	Roots  []frontend.Variable `gnark:",public"`
	Leaves []frontend.Variable `gnark:",public"`

	// private inputs
	InPathIndices  []frontend.Variable   `gnark:",secret"`
	InPathElements [][]frontend.Variable `gnark:",secret"`

	NumberOfCompressedAccounts uint32
	Height                     uint32
}

func (circuit *InclusionCircuit) Define(api frontend.API) error {
	abstractor.CallVoid(api, common.InclusionProof{
		Roots:          circuit.Roots,
		Leaves:         circuit.Leaves,
		InPathElements: circuit.InPathElements,
		InPathIndices:  circuit.InPathIndices,

		NumberOfCompressedAccounts: circuit.NumberOfCompressedAccounts,
		Height:                     circuit.Height,
	})
	return nil
}
