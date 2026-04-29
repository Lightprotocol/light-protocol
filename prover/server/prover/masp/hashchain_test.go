package masp

import (
	"bytes"
	"math/big"
	"testing"

	"light/light-prover/prover/masp/poseidon"
)

// beBytesToFe converts a 32-byte big-endian byte array to a *big.Int.
// Panics if the value is >= the BN254 Fr modulus (callers control inputs).
func beBytesToFe(b [32]byte) *big.Int {
	return new(big.Int).SetBytes(b[:])
}

func repeatByte(b byte) [32]byte {
	var out [32]byte
	for i := range out {
		out[i] = b
	}
	return out
}

// TestHashChainShape exercises the right-fold HashChain:
//
//  1. Length-2 input: coincides with light-protocol's left-fold (one hash
//     step, either direction). We keep the light-protocol golden here as
//     sanity.
//  2. Length-3 input: our right-fold (Poseidon2(x0, Poseidon2(x1, x2)))
//     differs from light-protocol's left-fold. We assert the right-fold
//     structure matches a manual nested computation.
//  3. Empty input: returns 0.
//  4. Single-element input: returns the element unchanged.
func TestHashChainShape(t *testing.T) {
	// Length-2 (coincides with light-protocol golden).
	in4 := repeatByte(4)
	in5 := repeatByte(5)
	want2 := [32]byte{
		13, 250, 206, 124, 182, 159, 160, 87, 57, 23, 80, 155, 25, 43, 40, 136,
		228, 255, 201, 1, 22, 168, 211, 220, 176, 187, 23, 176, 46, 198, 140, 211,
	}
	got := HashChain([]*big.Int{beBytesToFe(in4), beBytesToFe(in5)})
	if !bytes.Equal(gotBE(got), want2[:]) {
		t.Fatalf("HashChain(2-input) mismatch\n  want = %v\n  got  = %v", want2[:], gotBE(got))
	}

	// Length-3: Poseidon2(x0, Poseidon2(x1, x2)).
	in6 := repeatByte(6)
	tail := HashChain([]*big.Int{beBytesToFe(in5), beBytesToFe(in6)})
	manual3, _ := poseidon.HashWithT(3, []*big.Int{beBytesToFe(in4), tail})
	got3 := HashChain([]*big.Int{beBytesToFe(in4), beBytesToFe(in5), beBytesToFe(in6)})
	if got3.Cmp(manual3) != 0 {
		t.Fatalf("HashChain(3-input) != Poseidon2(x0, Poseidon2(x1, x2))")
	}

	// Empty input: zero.
	if HashChain(nil).Sign() != 0 {
		t.Fatal("HashChain(nil) expected zero")
	}

	// Single-input: returned as-is.
	if got := HashChain([]*big.Int{big.NewInt(42)}); got.Cmp(big.NewInt(42)) != 0 {
		t.Fatalf("HashChain([42]) expected 42, got %s", got)
	}
}

// gotBE returns the 32-byte big-endian representation of a field element,
// zero-padded on the left.
func gotBE(fe *big.Int) []byte {
	b := fe.Bytes()
	out := make([]byte, 32)
	copy(out[32-len(b):], b)
	return out
}

func TestTxHashStructure(t *testing.T) {
	// TxHash = HashChain([1, tx_blinding,
	//                       HashChain([c0, n0, c1, n1, ...]),
	//                       HashChain([o0, o1, ...])])
	c0 := big.NewInt(111)
	c1 := big.NewInt(222)
	n0 := big.NewInt(9991)
	n1 := big.NewInt(9992)
	o0 := big.NewInt(333)
	blinding := big.NewInt(444)

	inChain := HashChain([]*big.Int{c0, n0, c1, n1})
	outChain := HashChain([]*big.Int{o0})
	manual := HashChain([]*big.Int{DomainTxHash, blinding, inChain, outChain})

	got := TxHash([]*big.Int{c0, c1}, []*big.Int{n0, n1}, []*big.Int{o0}, blinding)
	if manual.Cmp(got) != 0 {
		t.Fatalf("TxHash != hierarchical manual chain\n  manual = %s\n  got    = %s", manual, got)
	}
}

func TestInNullifierChainInterleaves(t *testing.T) {
	c0 := big.NewInt(1)
	c1 := big.NewInt(2)
	n0 := big.NewInt(1001)
	n1 := big.NewInt(1002)
	want := HashChain([]*big.Int{c0, n0, c1, n1})
	got := InNullifierChain([]*big.Int{c0, c1}, []*big.Int{n0, n1})
	if got.Cmp(want) != 0 {
		t.Fatalf("InNullifierChain mismatch")
	}
}
