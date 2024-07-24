import ProvenZK
import FormalVerification.Poseidon
import FormalVerification.Circuit
import FormalVerification.Lemmas

open LightProver (F)

structure Range : Type where
  lo : Fin (2^248)
  hi : Fin (2^248)
  index : F

def Range.hash : Range → F := fun r => poseidon₃ vec![r.lo, r.index, r.hi]

def RangeTree (d : ℕ) : Type := { t: MerkleTree F poseidon₂ d // ∀ (i : Fin (2^d)), ∃ range, t.itemAtFin i = Range.hash range }

def rangeTreeMem {d} : Range → RangeTree d → Prop := fun r t => r.hash ∈ t.val

instance : Membership F Range where
  mem x r := r.lo.val < x.val ∧ x.val < r.hi.val

instance {d} : Membership F (RangeTree d) where
  mem x t := ∃(r:Range), rangeTreeMem r t ∧ x ∈ r
