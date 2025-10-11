package v1

import (
	"light/light-prover/prover/common"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/constraint"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
)

// R1CSInclusion creates the R1CS for V1 Inclusion circuit (without PublicInputHash)
func R1CSInclusion(treeHeight uint32, numberOfCompressedAccounts uint32) (constraint.ConstraintSystem, error) {
	roots := make([]frontend.Variable, numberOfCompressedAccounts)
	leaves := make([]frontend.Variable, numberOfCompressedAccounts)
	inPathIndices := make([]frontend.Variable, numberOfCompressedAccounts)
	inPathElements := make([][]frontend.Variable, numberOfCompressedAccounts)

	for i := 0; i < int(numberOfCompressedAccounts); i++ {
		inPathElements[i] = make([]frontend.Variable, treeHeight)
	}

	circuit := InclusionCircuit{
		Roots:                      roots,
		Leaves:                     leaves,
		InPathIndices:              inPathIndices,
		InPathElements:             inPathElements,
		NumberOfCompressedAccounts: numberOfCompressedAccounts,
		Height:                     treeHeight,
	}
	return frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &circuit)
}

// SetupInclusion creates proving system for V1 Inclusion circuit (without PublicInputHash)
func SetupInclusion(treeHeight uint32, numberOfCompressedAccounts uint32) (*common.MerkleProofSystem, error) {
	ccs, err := R1CSInclusion(treeHeight, numberOfCompressedAccounts)
	if err != nil {
		return nil, err
	}
	pk, vk, err := groth16.Setup(ccs)
	if err != nil {
		return nil, err
	}
	return &common.MerkleProofSystem{
		InclusionTreeHeight:                 treeHeight,
		InclusionNumberOfCompressedAccounts: numberOfCompressedAccounts,
		ProvingKey:                          pk,
		VerifyingKey:                        vk,
		ConstraintSystem:                    ccs,
		Version:                             1, // V1 circuits have version 1
	}, nil
}

// R1CSNonInclusion creates the R1CS for V1 NonInclusion circuit
func R1CSNonInclusion(treeHeight uint32, numberOfCompressedAccounts uint32) (constraint.ConstraintSystem, error) {
	roots := make([]frontend.Variable, numberOfCompressedAccounts)
	values := make([]frontend.Variable, numberOfCompressedAccounts)
	leafLowerRangeValues := make([]frontend.Variable, numberOfCompressedAccounts)
	leafHigherRangeValues := make([]frontend.Variable, numberOfCompressedAccounts)
	nextIndices := make([]frontend.Variable, numberOfCompressedAccounts)
	inPathIndices := make([]frontend.Variable, numberOfCompressedAccounts)
	inPathElements := make([][]frontend.Variable, numberOfCompressedAccounts)

	for i := 0; i < int(numberOfCompressedAccounts); i++ {
		inPathElements[i] = make([]frontend.Variable, treeHeight)
		// Initialize NextIndices with 0 to avoid nil values in auto-generated tests
		nextIndices[i] = 0
	}

	circuit := NonInclusionCircuit{
		Roots:                      roots,
		Values:                     values,
		LeafLowerRangeValues:       leafLowerRangeValues,
		LeafHigherRangeValues:      leafHigherRangeValues,
		NextIndices:                nextIndices,
		InPathIndices:              inPathIndices,
		InPathElements:             inPathElements,
		NumberOfCompressedAccounts: numberOfCompressedAccounts,
		Height:                     treeHeight,
	}
	return frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &circuit)
}

// SetupNonInclusion creates proving system for V1 NonInclusion circuit
func SetupNonInclusion(treeHeight uint32, numberOfCompressedAccounts uint32) (*common.MerkleProofSystem, error) {
	ccs, err := R1CSNonInclusion(treeHeight, numberOfCompressedAccounts)
	if err != nil {
		return nil, err
	}
	pk, vk, err := groth16.Setup(ccs)
	if err != nil {
		return nil, err
	}
	return &common.MerkleProofSystem{
		NonInclusionTreeHeight:                 treeHeight,
		NonInclusionNumberOfCompressedAccounts: numberOfCompressedAccounts,
		ProvingKey:                             pk,
		VerifyingKey:                           vk,
		ConstraintSystem:                       ccs,
		Version:                                1, // V1 circuits have version 1
	}, nil
}

// R1CSCombined creates the R1CS for V1 Combined circuit
func R1CSCombined(inclusionTreeHeight uint32, inclusionNumberOfCompressedAccounts uint32, nonInclusionTreeHeight uint32, nonInclusionNumberOfCompressedAccounts uint32) (constraint.ConstraintSystem, error) {
	circuit := InitializeCombinedCircuit(
		inclusionTreeHeight,
		inclusionNumberOfCompressedAccounts,
		nonInclusionTreeHeight,
		nonInclusionNumberOfCompressedAccounts,
	)
	return frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &circuit)
}

// SetupCombined creates proving system for V1 Combined circuit
func SetupCombined(inclusionTreeHeight uint32, inclusionNumberOfCompressedAccounts uint32, nonInclusionTreeHeight uint32, nonInclusionNumberOfCompressedAccounts uint32) (*common.MerkleProofSystem, error) {
	ccs, err := R1CSCombined(inclusionTreeHeight, inclusionNumberOfCompressedAccounts, nonInclusionTreeHeight, nonInclusionNumberOfCompressedAccounts)
	if err != nil {
		return nil, err
	}
	pk, vk, err := groth16.Setup(ccs)
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
		Version:                                1, // V1 circuits have version 1
	}, nil
}

func ImportInclusionSetup(treeHeight uint32, numberOfCompressedAccounts uint32, pkPath string, vkPath string, r1csPath string) (*common.MerkleProofSystem, error) {
	pk, err := common.LoadProvingKey(pkPath)
	if err != nil {
		return nil, err
	}

	vk, err := common.LoadVerifyingKey(vkPath)
	if err != nil {
		return nil, err
	}

	// Regenerate constraint system to match witness generation
	// The ceremony R1CS has a different variable layout than runtime witness generation
	ccs, err := R1CSInclusion(treeHeight, numberOfCompressedAccounts)
	if err != nil {
		return nil, err
	}

	return &common.MerkleProofSystem{
		InclusionTreeHeight:                 treeHeight,
		InclusionNumberOfCompressedAccounts: numberOfCompressedAccounts,
		ProvingKey:                          pk,
		VerifyingKey:                        vk,
		ConstraintSystem:                    ccs,
		Version:                             1,
	}, nil
}

func ImportNonInclusionSetup(treeHeight uint32, numberOfCompressedAccounts uint32, pkPath string, vkPath string, r1csPath string) (*common.MerkleProofSystem, error) {
	pk, err := common.LoadProvingKey(pkPath)
	if err != nil {
		return nil, err
	}

	vk, err := common.LoadVerifyingKey(vkPath)
	if err != nil {
		return nil, err
	}

	// Regenerate constraint system to match witness generation
	// The ceremony R1CS has a different variable layout than runtime witness generation
	ccs, err := R1CSNonInclusion(treeHeight, numberOfCompressedAccounts)
	if err != nil {
		return nil, err
	}

	return &common.MerkleProofSystem{
		NonInclusionTreeHeight:                 treeHeight,
		NonInclusionNumberOfCompressedAccounts: numberOfCompressedAccounts,
		ProvingKey:                             pk,
		VerifyingKey:                           vk,
		ConstraintSystem:                       ccs,
		Version:                                1,
	}, nil
}

func ImportCombinedSetup(inclusionTreeHeight uint32, inclusionNumberOfCompressedAccounts uint32, nonInclusionTreeHeight uint32, nonInclusionNumberOfCompressedAccounts uint32, pkPath string, vkPath string, r1csPath string) (*common.MerkleProofSystem, error) {
	pk, err := common.LoadProvingKey(pkPath)
	if err != nil {
		return nil, err
	}

	vk, err := common.LoadVerifyingKey(vkPath)
	if err != nil {
		return nil, err
	}

	// Regenerate constraint system to match witness generation
	// The ceremony R1CS has a different variable layout than runtime witness generation
	ccs, err := R1CSCombined(inclusionTreeHeight, inclusionNumberOfCompressedAccounts, nonInclusionTreeHeight, nonInclusionNumberOfCompressedAccounts)
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
		Version:                                1,
	}, nil
}
