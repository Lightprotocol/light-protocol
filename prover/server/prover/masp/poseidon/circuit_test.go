package poseidon

import (
	"math/big"
	"strconv"
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
	"github.com/consensys/gnark/test"
)

// equivCircuit asserts that the in-circuit hash of Inputs produces Expected.
// T is fixed per-instance because gnark circuits are monomorphic in their
// frontend.Variable slice lengths.
type equivCircuit struct {
	T        int `gnark:"-"`
	Inputs   []frontend.Variable
	Expected frontend.Variable `gnark:",public"`
}

func (c *equivCircuit) Define(api frontend.API) error {
	got := HashCircuitWithT(api, c.T, c.Inputs)
	api.AssertIsEqual(got, c.Expected)
	return nil
}

func TestCircuitEqualsNative(t *testing.T) {
	for width := MinWidth; width <= MaxWidth; width++ {
		width := width
		t.Run("t="+strconv.Itoa(width), func(t *testing.T) {
			inputs := make([]*big.Int, width-1)
			for i := range inputs {
				inputs[i] = big.NewInt(int64(i + 1))
			}
			native, err := HashWithT(width, inputs)
			if err != nil {
				t.Fatalf("native hash: %v", err)
			}

			assignInputs := make([]frontend.Variable, width-1)
			for i, in := range inputs {
				assignInputs[i] = in
			}

			assignment := &equivCircuit{
				T:        width,
				Inputs:   assignInputs,
				Expected: native,
			}
			circuit := &equivCircuit{
				T:      width,
				Inputs: make([]frontend.Variable, width-1),
			}

			err = test.IsSolved(circuit, assignment, ecc.BN254.ScalarField())
			if err != nil {
				t.Fatalf("circuit solve: %v", err)
			}
		})
	}
}

// TestCircuitCompiles exercises the actual R1CS compiler path for a
// representative width, catching gnark-version API breakage that IsSolved
// would miss.
func TestCircuitCompiles(t *testing.T) {
	circuit := &equivCircuit{
		T:      3,
		Inputs: make([]frontend.Variable, 2),
	}
	_, err := frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, circuit, frontend.WithCompressThreshold(300))
	if err != nil {
		t.Fatalf("compile: %v", err)
	}
}
