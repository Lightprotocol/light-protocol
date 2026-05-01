package masp

import (
	"math/big"
	"testing"

	"light/light-prover/prover/masp/poseidon"
)

func TestUtxoHashMatchesDirectPoseidon(t *testing.T) {
	u := Utxo{
		Owner:    big.NewInt(1),
		Spl:      big.NewInt(100),
		Sol:      big.NewInt(200),
		Blinding: big.NewInt(12345),
		DataHash: big.NewInt(0),
	}
	got := UtxoHash(u)

	want, err := poseidon.HashWithT(7, []*big.Int{
		big.NewInt(0), // DomainUtxo
		u.Owner,
		u.Spl,
		u.Sol,
		u.Blinding,
		u.DataHash,
	})
	if err != nil {
		t.Fatal(err)
	}
	if got.Cmp(want) != 0 {
		t.Fatalf("UtxoHash mismatch\n  got  = %s\n  want = %s", got, want)
	}
}

func TestUtxoHashDomainSeparation(t *testing.T) {
	// Changing any single field must change the digest.
	base := Utxo{
		Owner:    big.NewInt(1),
		Spl:      big.NewInt(100),
		Sol:      big.NewInt(200),
		Blinding: big.NewInt(12345),
		DataHash: big.NewInt(0),
	}
	baseHash := UtxoHash(base)

	mutators := []func(u *Utxo){
		func(u *Utxo) { u.Owner = big.NewInt(2) },
		func(u *Utxo) { u.Spl = big.NewInt(101) },
		func(u *Utxo) { u.Sol = big.NewInt(201) },
		func(u *Utxo) { u.Blinding = big.NewInt(12346) },
		func(u *Utxo) { u.DataHash = big.NewInt(1) },
	}
	for i, m := range mutators {
		u := base
		m(&u)
		if UtxoHash(u).Cmp(baseHash) == 0 {
			t.Fatalf("mutator %d did not change the digest", i)
		}
	}
}

func TestLocalDevShieldedUtxoVector(t *testing.T) {
	programID := new(big.Int).SetBytes([]byte{
		0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10,
	})
	seed := big.NewInt(0x1001)
	ownerHash, err := poseidon.HashWithT(3, []*big.Int{programID, seed})
	if err != nil {
		t.Fatal(err)
	}

	dataHash, err := poseidon.HashWithT(3, []*big.Int{
		repeatedByteField(0xBB),
		repeatedByteField(0x77),
	})
	if err != nil {
		t.Fatal(err)
	}
	assertBigHex(t, "data_hash", dataHash, "17fa8974fff8ca1375a813457f3f78fc34d4ebd922ba94120b0e67ad2cece6b6")

	utxoHash := UtxoHash(Utxo{
		Owner:    ownerHash,
		Spl:      big.NewInt(1_000_000),
		Sol:      big.NewInt(42),
		Blinding: repeatedByteField(0xCC),
		DataHash: dataHash,
	})
	assertBigHex(t, "utxo_hash", utxoHash, "12c96ee6a45c2661acebcf08a8a1b3331c343fc26797ced0f490f6fa219b0416")

	encryptedUtxoHash, err := poseidon.HashWithT(2, []*big.Int{utxoHash})
	if err != nil {
		t.Fatal(err)
	}
	assertBigHex(t, "encrypted_utxo_hash", encryptedUtxoHash, "23eb619b8ad0060898a4ee73676d28ba465d1a6fb8bdf15c1191ca4e553d7cd2")
}

func TestNullifyChain(t *testing.T) {
	inCommit := big.NewInt(0xDEADBEEF)
	master := big.NewInt(0xCAFEBABE)
	leafIndex := big.NewInt(42)

	dns := DomainNullifierSecret(inCommit, master)
	nf := Nullify(inCommit, leafIndex, dns)

	// Recompute manually via poseidon to make sure the layering is right.
	dnsManual, err := poseidon.HashWithT(3, []*big.Int{inCommit, master})
	if err != nil {
		t.Fatal(err)
	}
	if dns.Cmp(dnsManual) != 0 {
		t.Fatalf("DomainNullifierSecret mismatch")
	}
	nfManual, err := poseidon.HashWithT(4, []*big.Int{inCommit, leafIndex, dnsManual})
	if err != nil {
		t.Fatal(err)
	}
	if nf.Cmp(nfManual) != 0 {
		t.Fatalf("Nullify mismatch")
	}

	// Changing any argument must change the nullifier.
	if Nullify(big.NewInt(0xDEADBEEE), leafIndex, dns).Cmp(nf) == 0 {
		t.Fatal("nullifier not sensitive to in_commit")
	}
	if Nullify(inCommit, big.NewInt(43), dns).Cmp(nf) == 0 {
		t.Fatal("nullifier not sensitive to leaf_index")
	}
	if Nullify(inCommit, leafIndex, big.NewInt(0)).Cmp(nf) == 0 {
		t.Fatal("nullifier not sensitive to dns")
	}
}

func repeatedByteField(value byte) *big.Int {
	var bytes [32]byte
	for i := range bytes {
		bytes[i] = value
	}
	bytes[0] = 0
	return new(big.Int).SetBytes(bytes[:])
}

func assertBigHex(t *testing.T, name string, got *big.Int, wantHex string) {
	t.Helper()
	want, ok := new(big.Int).SetString(wantHex, 16)
	if !ok {
		t.Fatalf("invalid %s hex test vector", name)
	}
	if got.Cmp(want) != 0 {
		t.Fatalf("%s mismatch\n  got  = %064x\n  want = %064x", name, got, want)
	}
}
