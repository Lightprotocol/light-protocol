package v1

import (
	"github.com/consensys/gnark/frontend"
	"github.com/reilabs/gnark-lean-extractor/v3/abstractor"
)

// NonInclusionCircuit is the v1 non-inclusion circuit definition
type NonInclusionCircuit struct {
	Roots  []frontend.Variable `gnark:",public"`
	Values []frontend.Variable `gnark:",public"`

	// private inputs
	LeafLowerRangeValues  []frontend.Variable `gnark:",secret"`
	LeafHigherRangeValues []frontend.Variable `gnark:",secret"`
	NextIndices           []frontend.Variable `gnark:",secret"`

	InPathIndices  []frontend.Variable   `gnark:",secret"`
	InPathElements [][]frontend.Variable `gnark:",secret"`

	NumberOfCompressedAccounts uint32
	Height                     uint32
}

func (circuit *NonInclusionCircuit) Define(api frontend.API) error {
	proof := LegacyNonInclusionProof{
		Roots:  circuit.Roots,
		Values: circuit.Values,

		LeafLowerRangeValues:  circuit.LeafLowerRangeValues,
		LeafHigherRangeValues: circuit.LeafHigherRangeValues,
		NextIndices:           circuit.NextIndices,

		InPathElements: circuit.InPathElements,
		InPathIndices:  circuit.InPathIndices,

		NumberOfCompressedAccounts: circuit.NumberOfCompressedAccounts,
		Height:                     circuit.Height,
	}
	abstractor.CallVoid(api, proof)
	return nil
}
