package masp

import (
	"math/big"

	"golang.org/x/crypto/sha3"

	"light/light-prover/prover/masp/poseidon"
)

var lightAccountDataDiscriminatorDomain = new(big.Int).Lsh(big.NewInt(2), 64)

var sampleShieldedPoolProgramID = [32]byte{
	0x7a, 0x6f, 0x6e, 0x65, 0x64, 0x2d, 0x73, 0x68,
	0x69, 0x65, 0x6c, 0x64, 0x65, 0x64, 0x2d, 0x70,
	0x6f, 0x6f, 0x6c, 0x2d, 0x70, 0x72, 0x6f, 0x67,
	0x72, 0x61, 0x6d, 0x2d, 0x76, 0x31, 0x00, 0x01,
}

var sampleUtxoTreeID = [32]byte{
	0x7a, 0x6f, 0x6e, 0x65, 0x64, 0x2d, 0x73, 0x68,
	0x69, 0x65, 0x6c, 0x64, 0x65, 0x64, 0x2d, 0x75,
	0x74, 0x78, 0x6f, 0x2d, 0x74, 0x72, 0x65, 0x65,
	0x2d, 0x76, 0x31, 0x00, 0x00, 0x00, 0x00, 0x01,
}

var sampleShieldedUtxoDiscriminator = [8]byte{'s', 'h', 'l', 'd', 'u', 't', 'x', '1'}

// HashToBn254FieldSizeBE mirrors light_hasher::hash_to_bn254_field_size_be:
// Keccak256(bytes || 0xff), then clear the top byte so the value fits BN254 Fr.
func HashToBn254FieldSizeBE(bytes []byte) *big.Int {
	h := sha3.NewLegacyKeccak256()
	h.Write(bytes)
	h.Write([]byte{0xff})
	sum := h.Sum(nil)
	sum[0] = 0
	return new(big.Int).SetBytes(sum)
}

func DiscriminatorToField(discriminator [8]byte) *big.Int {
	return new(big.Int).SetBytes(discriminator[:])
}

func SampleShieldedAccountOwnerHash() *big.Int {
	return HashToBn254FieldSizeBE(sampleShieldedPoolProgramID[:])
}

func SampleUtxoTreeHash() *big.Int {
	return HashToBn254FieldSizeBE(sampleUtxoTreeID[:])
}

func SampleShieldedUtxoDiscriminator() *big.Int {
	return DiscriminatorToField(sampleShieldedUtxoDiscriminator)
}

// LightCompressedAccountLeafHash computes the specialized Light compressed
// account hash used for shielded UTXO leaves:
//
//	Poseidon(owner_hash, leaf_index, tree_hash, 2||discriminator, utxo_commit)
//
// This is the no-lamports/no-address shape with account data present. It
// matches light_compressed_account::hash_with_hashed_values for batched trees
// when data_hash == utxo_commit.
func LightCompressedAccountLeafHash(ownerHash, leafIndex, treeHash, discriminator, utxoCommit *big.Int) *big.Int {
	discriminatorField := new(big.Int).Add(discriminator, lightAccountDataDiscriminatorDomain)
	h, err := poseidon.HashWithT(6, []*big.Int{
		ownerHash,
		leafIndex,
		treeHash,
		discriminatorField,
		utxoCommit,
	})
	if err != nil {
		panic(err)
	}
	return h
}
