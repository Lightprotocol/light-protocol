package poseidon

import (
	"fmt"
	"math/big"

	"github.com/consensys/gnark/frontend"
)

// HashCircuit computes Poseidon(0, inputs...) in-circuit at width t = len(inputs)+1.
// The result is a single field element. Domain tag is fixed at 0 (Circom convention).
func HashCircuit(api frontend.API, inputs []frontend.Variable) frontend.Variable {
	return HashCircuitWithT(api, len(inputs)+1, inputs)
}

// HashCircuitWithT computes Poseidon(0, inputs...) in-circuit at a specific width t.
// Requires len(inputs) == t-1.
func HashCircuitWithT(api frontend.API, t int, inputs []frontend.Variable) frontend.Variable {
	return HashCircuitWithDomainTag(api, t, frontend.Variable(0), inputs)
}

// HashCircuitWithDomainTag computes Poseidon(domainTag, inputs...) in-circuit at width t.
func HashCircuitWithDomainTag(api frontend.API, t int, domainTag frontend.Variable, inputs []frontend.Variable) frontend.Variable {
	if t < MinWidth || t > MaxWidth {
		panic(fmt.Sprintf("poseidon.HashCircuit: unsupported width t=%d", t))
	}
	if len(inputs) != t-1 {
		panic(fmt.Sprintf("poseidon.HashCircuit: want %d inputs for t=%d, got %d", t-1, t, len(inputs)))
	}
	state := make([]frontend.Variable, t)
	state[0] = domainTag
	for i, in := range inputs {
		state[i+1] = in
	}
	PermuteCircuit(api, state)
	return state[0]
}

// PermuteCircuit applies the Poseidon permutation in-circuit in place.
// len(state) must equal some supported width t.
func PermuteCircuit(api frontend.API, state []frontend.Variable) {
	t := len(state)
	permute(state, CFG[t], circuitArith{api: api})
}

// circuitArith is the arith[T=frontend.Variable] implementation backing
// PermuteCircuit. Field reduction is implicit in gnark's api.
type circuitArith struct{ api frontend.API }

func (c circuitArith) Add(a, b frontend.Variable) frontend.Variable { return c.api.Add(a, b) }
func (c circuitArith) Mul(a, b frontend.Variable) frontend.Variable { return c.api.Mul(a, b) }

// FromBig wraps a *big.Int constant as a frontend.Variable. gnark accepts
// *big.Int anywhere a Variable is expected, so this is a no-op cast.
func (circuitArith) FromBig(b *big.Int) frontend.Variable { return b }
