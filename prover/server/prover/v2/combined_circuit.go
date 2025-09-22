package v2

import (
	"light/light-prover/prover/common"
	"light/light-prover/prover/poseidon"

	"github.com/consensys/gnark/frontend"
	"github.com/reilabs/gnark-lean-extractor/v3/abstractor"
)

type CombinedCircuit struct {
	PublicInputHash frontend.Variable        `gnark:",public"`
	Inclusion       common.InclusionProof    `gnark:",secret"`
	NonInclusion    common.NonInclusionProof `gnark:",secret"`
}

func (circuit *CombinedCircuit) Define(api frontend.API) error {
	inclusionPublicInputsHashChain := createTwoInputsHashChain(api, circuit.Inclusion.Roots, circuit.Inclusion.Leaves)
	nonInclusionPublicInputsHashChain := createTwoInputsHashChain(api, circuit.NonInclusion.Roots, circuit.NonInclusion.Values)

	publicInputsHashChain := abstractor.Call(api, poseidon.Poseidon2{In1: inclusionPublicInputsHashChain, In2: nonInclusionPublicInputsHashChain})
	api.AssertIsEqual(circuit.PublicInputHash, publicInputsHashChain)

	abstractor.CallVoid(api, common.InclusionProof{
		Roots:          circuit.Inclusion.Roots,
		Leaves:         circuit.Inclusion.Leaves,
		InPathElements: circuit.Inclusion.InPathElements,
		InPathIndices:  circuit.Inclusion.InPathIndices,

		NumberOfCompressedAccounts: circuit.Inclusion.NumberOfCompressedAccounts,
		Height:                     circuit.Inclusion.Height,
	})

	proof := common.NonInclusionProof{
		Roots:  circuit.NonInclusion.Roots,
		Values: circuit.NonInclusion.Values,

		LeafLowerRangeValues:  circuit.NonInclusion.LeafLowerRangeValues,
		LeafHigherRangeValues: circuit.NonInclusion.LeafHigherRangeValues,

		InPathElements: circuit.NonInclusion.InPathElements,
		InPathIndices:  circuit.NonInclusion.InPathIndices,

		NumberOfCompressedAccounts: circuit.NonInclusion.NumberOfCompressedAccounts,
		Height:                     circuit.NonInclusion.Height,
	}
	abstractor.CallVoid(api, proof)
	return nil
}

func ImportCombinedSetup(inclusionTreeHeight uint32, inclusionNumberOfCompressedAccounts uint32, nonInclusionTreeHeight uint32, nonInclusionNumberOfCompressedAccounts uint32, pkPath string, vkPath string) (*common.MerkleProofSystem, error) {
	ccs, err := R1CSCombined(inclusionTreeHeight, inclusionNumberOfCompressedAccounts, nonInclusionTreeHeight, nonInclusionNumberOfCompressedAccounts)
	if err != nil {
		return nil, err
	}

	pk, err := common.LoadProvingKey(pkPath)

	if err != nil {
		return nil, err
	}

	vk, err := common.LoadVerifyingKey(vkPath)

	if err != nil {
		return nil, err
	}

	return &common.MerkleProofSystem{
		InclusionTreeHeight:                    inclusionTreeHeight,
		InclusionNumberOfCompressedAccounts:    inclusionNumberOfCompressedAccounts,
		NonInclusionTreeHeight:                 nonInclusionTreeHeight,
		NonInclusionNumberOfCompressedAccounts: nonInclusionNumberOfCompressedAccounts,
		ProvingKey:                             pk,
		VerifyingKey:                           vk,
		ConstraintSystem:                       ccs,
	}, nil
}
