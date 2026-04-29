// Package poseidon implements the BN254 Poseidon hash for widths t = 2..14,
// bit-compatible with light-poseidon's Circom convention (domain tag = 0, x^5
// S-box, RF/2 full + RP partial + RF/2 full rounds).
package poseidon

import (
	"errors"
	"fmt"
	"math/big"
)

// Modulus is the BN254 scalar field (Fr) modulus.
var Modulus, _ = new(big.Int).SetString("30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001", 16)

// MinWidth and MaxWidth are the inclusive bounds on the supported width t.
const (
	MinWidth = 2
	MaxWidth = 14
)

// FullRounds is the number of full rounds for every width.
const FullRounds = 8

// FieldBytes is the canonical byte length of a Fr element (32 bytes).
const FieldBytes = 32

// Cfg holds the Poseidon parameters for a given width.
type Cfg struct {
	RF  int
	RP  int
	ARK [][]*big.Int // shape: (RF+RP) x t
	MDS [][]*big.Int // shape: t x t
}

// CFG indexes configurations by width. CFG[t] is valid for t in [MinWidth, MaxWidth].
var CFG [MaxWidth + 1]*Cfg

func init() {
	register(2, ARK_2, MDS_2)
	register(3, ARK_3, MDS_3)
	register(4, ARK_4, MDS_4)
	register(5, ARK_5, MDS_5)
	register(6, ARK_6, MDS_6)
	register(7, ARK_7, MDS_7)
	register(8, ARK_8, MDS_8)
	register(9, ARK_9, MDS_9)
	register(10, ARK_10, MDS_10)
	register(11, ARK_11, MDS_11)
	register(12, ARK_12, MDS_12)
	register(13, ARK_13, MDS_13)
	register(14, ARK_14, MDS_14)
}

func register(t int, flatArk []*big.Int, mds [][]*big.Int) {
	rp := PARTIAL_ROUNDS[t-2]
	rf := FullRounds
	totalRounds := rf + rp
	if len(flatArk) != totalRounds*t {
		panic(fmt.Sprintf("poseidon: ARK length mismatch for t=%d: got %d, want %d", t, len(flatArk), totalRounds*t))
	}
	if len(mds) != t {
		panic(fmt.Sprintf("poseidon: MDS row count mismatch for t=%d", t))
	}
	for i, row := range mds {
		if len(row) != t {
			panic(fmt.Sprintf("poseidon: MDS row %d length mismatch for t=%d", i, t))
		}
	}
	ark := make([][]*big.Int, totalRounds)
	for r := 0; r < totalRounds; r++ {
		ark[r] = flatArk[r*t : (r+1)*t]
	}
	CFG[t] = &Cfg{
		RF:  rf,
		RP:  rp,
		ARK: ark,
		MDS: mds,
	}
}

// Errors returned by the hash API.
var (
	ErrEmptyInput       = errors.New("poseidon: empty input")
	ErrInvalidWidth     = errors.New("poseidon: unsupported width (must be 2..14)")
	ErrWrongInputLength = errors.New("poseidon: input length must equal t-1")
	ErrByteLength       = errors.New("poseidon: each byte input must be exactly 32 bytes")
	ErrInputOverModulus = errors.New("poseidon: input >= field modulus")
)

// Hash computes Poseidon(0, inputs...) at width t = len(inputs)+1, returning the
// digest as a field element. This is the Circom-compatible path.
func Hash(inputs []*big.Int) (*big.Int, error) {
	return HashWithT(len(inputs)+1, inputs)
}

// HashWithT computes Poseidon(domain_tag=0, inputs...) at a specific width t.
// Requires len(inputs) == t-1.
func HashWithT(t int, inputs []*big.Int) (*big.Int, error) {
	return HashWithDomainTag(t, new(big.Int), inputs)
}

// HashWithDomainTag computes Poseidon(domain_tag, inputs...) at width t.
func HashWithDomainTag(t int, domainTag *big.Int, inputs []*big.Int) (*big.Int, error) {
	if t < MinWidth || t > MaxWidth {
		return nil, ErrInvalidWidth
	}
	if len(inputs) != t-1 {
		return nil, ErrWrongInputLength
	}
	for _, in := range inputs {
		if in == nil {
			return nil, ErrEmptyInput
		}
		if in.Sign() < 0 || in.Cmp(Modulus) >= 0 {
			return nil, ErrInputOverModulus
		}
	}
	if domainTag == nil {
		return nil, ErrEmptyInput
	}
	if domainTag.Sign() < 0 || domainTag.Cmp(Modulus) >= 0 {
		return nil, ErrInputOverModulus
	}
	state := make([]*big.Int, t)
	state[0] = new(big.Int).Set(domainTag)
	for i, in := range inputs {
		state[i+1] = new(big.Int).Set(in)
	}
	Permute(state)
	return state[0], nil
}

// HashBytesBE hashes t-1 big-endian 32-byte inputs and returns the digest as a
// 32-byte big-endian array. Each input must be exactly 32 bytes.
func HashBytesBE(inputs [][]byte) ([FieldBytes]byte, error) {
	var zero [FieldBytes]byte
	fes, err := bytesInputsToFe(inputs, true)
	if err != nil {
		return zero, err
	}
	out, err := Hash(fes)
	if err != nil {
		return zero, err
	}
	return feToBytesBE(out), nil
}

// HashBytesLE hashes t-1 little-endian 32-byte inputs and returns the digest as
// a 32-byte little-endian array.
func HashBytesLE(inputs [][]byte) ([FieldBytes]byte, error) {
	var zero [FieldBytes]byte
	fes, err := bytesInputsToFe(inputs, false)
	if err != nil {
		return zero, err
	}
	out, err := Hash(fes)
	if err != nil {
		return zero, err
	}
	return feToBytesLE(out), nil
}

func bytesInputsToFe(inputs [][]byte, bigEndian bool) ([]*big.Int, error) {
	if len(inputs) == 0 {
		return nil, ErrEmptyInput
	}
	out := make([]*big.Int, len(inputs))
	for i, b := range inputs {
		if len(b) != FieldBytes {
			return nil, ErrByteLength
		}
		fe := new(big.Int)
		if bigEndian {
			fe.SetBytes(b)
		} else {
			buf := make([]byte, FieldBytes)
			for j := 0; j < FieldBytes; j++ {
				buf[j] = b[FieldBytes-1-j]
			}
			fe.SetBytes(buf)
		}
		if fe.Cmp(Modulus) >= 0 {
			return nil, ErrInputOverModulus
		}
		out[i] = fe
	}
	return out, nil
}

func feToBytesBE(fe *big.Int) [FieldBytes]byte {
	var out [FieldBytes]byte
	b := fe.Bytes()
	copy(out[FieldBytes-len(b):], b)
	return out
}

func feToBytesLE(fe *big.Int) [FieldBytes]byte {
	be := feToBytesBE(fe)
	var out [FieldBytes]byte
	for i := 0; i < FieldBytes; i++ {
		out[i] = be[FieldBytes-1-i]
	}
	return out
}

// Permute applies the Poseidon permutation in place. len(state) must equal the
// width t for some supported t. Each element must already be reduced mod p.
func Permute(state []*big.Int) {
	t := len(state)
	permute(state, CFG[t], nativeArith{})
}

// nativeArith is the arith[T=*big.Int] implementation backing Permute. Add and
// Mul allocate a fresh *big.Int per call so the round-constant and MDS inputs
// passed through FromBig are never mutated.
type nativeArith struct{}

func (nativeArith) Add(a, b *big.Int) *big.Int {
	r := new(big.Int).Add(a, b)
	r.Mod(r, Modulus)
	return r
}

func (nativeArith) Mul(a, b *big.Int) *big.Int {
	r := new(big.Int).Mul(a, b)
	r.Mod(r, Modulus)
	return r
}

// FromBig returns b directly: Add/Mul never mutate their operands, and ARK/MDS
// constants are already reduced mod p at init time.
func (nativeArith) FromBig(b *big.Int) *big.Int { return b }
