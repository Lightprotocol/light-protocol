import FormalVerification
import ProvenZk.Binary
import ProvenZk.Hash
import ProvenZk.Merkle
import ProvenZk.Ext.Vector

def Bn256_Fr : Nat := 0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001
axiom bn256_Fr_prime : Nat.Prime Bn256_Fr
instance : Fact (Nat.Prime SemaphoreMTB.Order) := Fact.mk (by apply bn256_Fr_prime)

abbrev D := 30
abbrev B := 4
