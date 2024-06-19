package prover

import (
	"github.com/consensys/gnark/frontend"
	"github.com/reilabs/gnark-lean-extractor/v2/abstractor"
)

type CombinedCircuit struct {
	Inclusion    InclusionCircuit
	NonInclusion NonInclusionCircuit
}

func (circuit *CombinedCircuit) Define(api frontend.API) error {
	abstractor.CallVoid(api, InclusionProof{
		Roots:                      circuit.Inclusion.Roots,
		Leaves:                     circuit.Inclusion.Leaves,
		InPathElements:             circuit.Inclusion.InPathElements,
		InPathIndices:              circuit.Inclusion.InPathIndices,
		NumberOfCompressedAccounts: circuit.Inclusion.NumberOfCompressedAccounts,
		Depth:                      circuit.Inclusion.Depth,
	})

	abstractor.CallVoid(api, NonInclusionProof{
		Roots:                      circuit.NonInclusion.Roots,
		Values:                     circuit.NonInclusion.Values,
		LeafLowerRangeValues:       circuit.NonInclusion.LeafLowerRangeValues,
		LeafHigherRangeValues:      circuit.NonInclusion.LeafHigherRangeValues,
		NextIndices:                circuit.NonInclusion.NextIndices,
		InPathIndices:              circuit.NonInclusion.InPathIndices,
		InPathElements:             circuit.NonInclusion.InPathElements,
		NumberOfCompressedAccounts: circuit.NonInclusion.NumberOfCompressedAccounts,
		Depth:                      circuit.NonInclusion.Depth,
	})
	return nil
}

func ImportCombinedSetup(inclusionTreeDepth uint32, inclusionNumberOfCompressedAccounts uint32, nonInclusionTreeDepth uint32, nonInclusionNumberOfCompressedAccounts uint32, pkPath string, vkPath string) (*ProvingSystem, error) {
	ccs, err := R1CSCombined(inclusionTreeDepth, inclusionNumberOfCompressedAccounts, nonInclusionTreeDepth, nonInclusionNumberOfCompressedAccounts)
	if err != nil {
		return nil, err
	}

	pk, err := LoadProvingKey(pkPath)

	if err != nil {
		return nil, err
	}

	vk, err := LoadVerifyingKey(vkPath)

	if err != nil {
		return nil, err
	}

	return &ProvingSystem{inclusionTreeDepth, inclusionNumberOfCompressedAccounts, nonInclusionTreeDepth, nonInclusionNumberOfCompressedAccounts, pk, vk, ccs}, nil
}
