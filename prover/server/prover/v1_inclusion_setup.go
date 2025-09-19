package prover

import (
	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/constraint"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
)

// R1CSV1Inclusion creates the R1CS for V1 Inclusion circuit (without PublicInputHash)
func R1CSV1Inclusion(treeHeight uint32, numberOfCompressedAccounts uint32) (constraint.ConstraintSystem, error) {
	roots := make([]frontend.Variable, numberOfCompressedAccounts)
	leaves := make([]frontend.Variable, numberOfCompressedAccounts)
	inPathIndices := make([]frontend.Variable, numberOfCompressedAccounts)
	inPathElements := make([][]frontend.Variable, numberOfCompressedAccounts)

	for i := 0; i < int(numberOfCompressedAccounts); i++ {
		inPathElements[i] = make([]frontend.Variable, treeHeight)
	}

	circuit := V1InclusionCircuit{
		Roots:                      roots,
		Leaves:                     leaves,
		InPathIndices:              inPathIndices,
		InPathElements:             inPathElements,
		NumberOfCompressedAccounts: numberOfCompressedAccounts,
		Height:                     treeHeight,
	}
	return frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &circuit)
}

// SetupV1Inclusion creates proving system for V1 Inclusion circuit (without PublicInputHash)
// This is used for mainnet_inclusion_26_* keys
func SetupV1Inclusion(treeHeight uint32, numberOfCompressedAccounts uint32) (*ProvingSystemV1, error) {
	ccs, err := R1CSV1Inclusion(treeHeight, numberOfCompressedAccounts)
	if err != nil {
		return nil, err
	}
	pk, vk, err := groth16.Setup(ccs)
	if err != nil {
		return nil, err
	}
	return &ProvingSystemV1{
		InclusionTreeHeight:                 treeHeight,
		InclusionNumberOfCompressedAccounts: numberOfCompressedAccounts,
		ProvingKey:                          pk,
		VerifyingKey:                        vk,
		ConstraintSystem:                    ccs,
		Version:                             0, // V1 circuits have version 0
	}, nil
}

// R1CSV1NonInclusion creates the R1CS for V1 NonInclusion circuit
func R1CSV1NonInclusion(treeHeight uint32, numberOfCompressedAccounts uint32) (constraint.ConstraintSystem, error) {
	roots := make([]frontend.Variable, numberOfCompressedAccounts)
	values := make([]frontend.Variable, numberOfCompressedAccounts)
	leafLowerRangeValues := make([]frontend.Variable, numberOfCompressedAccounts)
	leafHigherRangeValues := make([]frontend.Variable, numberOfCompressedAccounts)
	leafIndices := make([]frontend.Variable, numberOfCompressedAccounts)
	inPathIndices := make([]frontend.Variable, numberOfCompressedAccounts)
	inPathElements := make([][]frontend.Variable, numberOfCompressedAccounts)

	for i := 0; i < int(numberOfCompressedAccounts); i++ {
		inPathElements[i] = make([]frontend.Variable, treeHeight)
	}

	circuit := V1NonInclusionCircuit{
		Roots:                      roots,
		Values:                     values,
		LeafLowerRangeValues:       leafLowerRangeValues,
		LeafHigherRangeValues:      leafHigherRangeValues,
		NextIndices:                leafIndices,
		InPathIndices:              inPathIndices,
		InPathElements:             inPathElements,
		NumberOfCompressedAccounts: numberOfCompressedAccounts,
		Height:                     treeHeight,
	}
	return frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &circuit)
}

// SetupV1NonInclusion creates proving system for V1 NonInclusion circuit
// This is used for non-inclusion_26_* keys
func SetupV1NonInclusion(treeHeight uint32, numberOfCompressedAccounts uint32) (*ProvingSystemV1, error) {
	ccs, err := R1CSV1NonInclusion(treeHeight, numberOfCompressedAccounts)
	if err != nil {
		return nil, err
	}
	pk, vk, err := groth16.Setup(ccs)
	if err != nil {
		return nil, err
	}
	return &ProvingSystemV1{
		NonInclusionTreeHeight:                 treeHeight,
		NonInclusionNumberOfCompressedAccounts: numberOfCompressedAccounts,
		ProvingKey:                             pk,
		VerifyingKey:                           vk,
		ConstraintSystem:                       ccs,
		Version:                                0, // V1 circuits have version 0
	}, nil
}

// R1CSV1Combined creates the R1CS for V1 Combined circuit
func R1CSV1Combined(inclusionTreeHeight uint32, inclusionNumberOfCompressedAccounts uint32, nonInclusionTreeHeight uint32, nonInclusionNumberOfCompressedAccounts uint32) (constraint.ConstraintSystem, error) {
	circuit := V1InitializeCombinedCircuit(
		inclusionTreeHeight,
		inclusionNumberOfCompressedAccounts,
		nonInclusionTreeHeight,
		nonInclusionNumberOfCompressedAccounts,
	)
	return frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &circuit)
}

// SetupV1Combined creates proving system for V1 Combined circuit
// This is used for combined_26_* keys
func SetupV1Combined(inclusionTreeHeight uint32, inclusionNumberOfCompressedAccounts uint32, nonInclusionTreeHeight uint32, nonInclusionNumberOfCompressedAccounts uint32) (*ProvingSystemV1, error) {
	ccs, err := R1CSV1Combined(inclusionTreeHeight, inclusionNumberOfCompressedAccounts, nonInclusionTreeHeight, nonInclusionNumberOfCompressedAccounts)
	if err != nil {
		return nil, err
	}
	pk, vk, err := groth16.Setup(ccs)
	if err != nil {
		return nil, err
	}
	return &ProvingSystemV1{
		InclusionTreeHeight:                     inclusionTreeHeight,
		InclusionNumberOfCompressedAccounts:     inclusionNumberOfCompressedAccounts,
		NonInclusionTreeHeight:                  nonInclusionTreeHeight,
		NonInclusionNumberOfCompressedAccounts: nonInclusionNumberOfCompressedAccounts,
		ProvingKey:                              pk,
		VerifyingKey:                            vk,
		ConstraintSystem:                        ccs,
		Version:                                 0, // V1 circuits have version 0
	}, nil
}