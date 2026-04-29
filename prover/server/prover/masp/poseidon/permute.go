package poseidon

import (
	"fmt"
	"math/big"
)

// arith abstracts field operations so native (*big.Int with explicit mod) and
// gnark circuit (frontend.Variable) code paths share a single permutation
// implementation.
type arith[T any] interface {
	Add(a, b T) T
	Mul(a, b T) T
	// FromBig converts a *big.Int (round constant or MDS entry) into T.
	// Implementations may return the argument directly if Add/Mul do not
	// mutate their operands.
	FromBig(b *big.Int) T
}

func permute[T any](state []T, cfg *Cfg, ops arith[T]) {
	t := len(state)
	if cfg == nil {
		panic(fmt.Sprintf("poseidon: unsupported width t=%d", t))
	}
	halfFull := cfg.RF / 2

	for r := 0; r < halfFull; r++ {
		addArk(state, cfg.ARK[r], ops)
		sboxFull(state, ops)
		applyMDSGeneric(state, cfg.MDS, ops)
	}
	for r := halfFull; r < halfFull+cfg.RP; r++ {
		addArk(state, cfg.ARK[r], ops)
		sboxPartial(state, ops)
		applyMDSGeneric(state, cfg.MDS, ops)
	}
	for r := halfFull + cfg.RP; r < cfg.RF+cfg.RP; r++ {
		addArk(state, cfg.ARK[r], ops)
		sboxFull(state, ops)
		applyMDSGeneric(state, cfg.MDS, ops)
	}
}

func addArk[T any](state []T, round []*big.Int, ops arith[T]) {
	for i, s := range state {
		state[i] = ops.Add(s, ops.FromBig(round[i]))
	}
}

func sboxFull[T any](state []T, ops arith[T]) {
	for i, s := range state {
		state[i] = sboxGeneric(s, ops)
	}
}

func sboxPartial[T any](state []T, ops arith[T]) {
	state[0] = sboxGeneric(state[0], ops)
}

func sboxGeneric[T any](x T, ops arith[T]) T {
	x2 := ops.Mul(x, x)
	x4 := ops.Mul(x2, x2)
	return ops.Mul(x4, x)
}

func applyMDSGeneric[T any](state []T, mds [][]*big.Int, ops arith[T]) {
	t := len(state)
	next := make([]T, t)
	for i := 0; i < t; i++ {
		sum := ops.Mul(state[0], ops.FromBig(mds[i][0]))
		for j := 1; j < t; j++ {
			term := ops.Mul(state[j], ops.FromBig(mds[i][j]))
			sum = ops.Add(sum, term)
		}
		next[i] = sum
	}
	copy(state, next)
}
