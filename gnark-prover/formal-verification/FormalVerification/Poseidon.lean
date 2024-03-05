import FormalVerification
import FormalVerification.Common
import Mathlib
import ProvenZk

open SemaphoreMTB (F Order)

instance : Fact (Nat.Prime SemaphoreMTB.Order) := Fact.mk (by apply bn256_Fr_prime)

def sbox_uniqueAssignment (Inp : F): UniqueAssignment (SemaphoreMTB.sbox Inp) id := UniqueAssignment.mk _ $ by
  simp [SemaphoreMTB.sbox]; tauto

def mds_3_uniqueAssignment (S : Vector F 3): UniqueAssignment (SemaphoreMTB.mds_3 S) id := UniqueAssignment.mk _ $ by
  simp [SemaphoreMTB.mds_3]; tauto

def fullRound_3_3_uniqueAssignment (S C : Vector F 3): UniqueAssignment (SemaphoreMTB.fullRound_3_3 S C) id := UniqueAssignment.mk _ $ by
  simp [SemaphoreMTB.fullRound_3_3, (sbox_uniqueAssignment _).equiv, (mds_3_uniqueAssignment _).equiv]; tauto

def halfRound_3_3_uniqueAssignment (S C : Vector F 3): UniqueAssignment (SemaphoreMTB.halfRound_3_3 S C) id := UniqueAssignment.mk _ $ by
  simp [SemaphoreMTB.halfRound_3_3, (sbox_uniqueAssignment _).equiv, (mds_3_uniqueAssignment _).equiv]; tauto

def poseidon_3_uniqueAssignment (inp : Vector F 3): UniqueAssignment (SemaphoreMTB.poseidon_3 inp) id := by
  unfold SemaphoreMTB.poseidon_3
  repeat (
    apply UniqueAssignment.compose
    . (first | apply fullRound_3_3_uniqueAssignment | apply halfRound_3_3_uniqueAssignment)
    intro _
  )
  apply UniqueAssignment.constant

theorem poseidon_3_testVector : (poseidon_3_uniqueAssignment (vec![0,1,2])).val = vec![0x115cc0f5e7d690413df64c6b9662e9cf2a3617f2743245519e19607a4417189a, 0x0fca49b798923ab0239de1c9e7a4a9a2210312b6a2f616d18b5a87f9b628ae29, 0x0e7ae82e40091e63cbd4f16a6d16310b3729d4b6e138fcf54110e2867045a30c] :=
  by native_decide

def poseidon₂ : Hash F 2 := fun a => (poseidon_3_uniqueAssignment vec![0, a.get 0, a.get 1]).val.get 0

lemma Poseidon2_uncps (a b : F) (k : F -> Prop) : SemaphoreMTB.Poseidon2 a b k ↔ k (poseidon₂ vec![a, b]) := by
    unfold SemaphoreMTB.Poseidon2 poseidon₂
    apply Iff.of_eq
    rw [(poseidon_3_uniqueAssignment _).equiv]
    congr
