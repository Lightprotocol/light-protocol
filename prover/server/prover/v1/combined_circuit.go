package v1

import (
	"light/light-prover/prover/common"

	"github.com/consensys/gnark/frontend"
	"github.com/reilabs/gnark-lean-extractor/v3/abstractor"
)

// CombinedCircuit combines inclusion and non-inclusion circuits for v1
type CombinedCircuit struct {
	Inclusion    InclusionCircuit
	NonInclusion NonInclusionCircuit
}

func (circuit *CombinedCircuit) Define(api frontend.API) error {
	abstractor.CallVoid(api, common.InclusionProof{
		Roots:          circuit.Inclusion.Roots,
		Leaves:         circuit.Inclusion.Leaves,
		InPathElements: circuit.Inclusion.InPathElements,
		InPathIndices:  circuit.Inclusion.InPathIndices,

		NumberOfCompressedAccounts: circuit.Inclusion.NumberOfCompressedAccounts,
		Height:                     circuit.Inclusion.Height,
	})

	proof := LegacyNonInclusionProof{
		Roots:  circuit.NonInclusion.Roots,
		Values: circuit.NonInclusion.Values,

		LeafLowerRangeValues:  circuit.NonInclusion.LeafLowerRangeValues,
		LeafHigherRangeValues: circuit.NonInclusion.LeafHigherRangeValues,
		NextIndices:           circuit.NonInclusion.NextIndices,

		InPathElements: circuit.NonInclusion.InPathElements,
		InPathIndices:  circuit.NonInclusion.InPathIndices,

		NumberOfCompressedAccounts: circuit.NonInclusion.NumberOfCompressedAccounts,
		Height:                     circuit.NonInclusion.Height,
	}
	abstractor.CallVoid(api, proof)
	return nil
}
