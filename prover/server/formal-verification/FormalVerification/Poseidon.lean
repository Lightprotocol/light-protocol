import FormalVerification.Circuit
import FormalVerification.Lemmas
import Mathlib
import «ProvenZk»

open LightProver (F Order)

def sbox_uniqueAssignment (Inp : F): UniqueAssignment (LightProver.sbox Inp) id := UniqueAssignment.mk _ $ by
  simp [LightProver.sbox]; tauto

def mds_3_uniqueAssignment (S : List.Vector F 3): UniqueAssignment (LightProver.mds_3 S) id := UniqueAssignment.mk _ $ by
  simp [LightProver.mds_3]; tauto

def fullRound_3_3_uniqueAssignment (S C : List.Vector F 3): UniqueAssignment (LightProver.fullRound_3_3 S C) id := UniqueAssignment.mk _ $ by
  simp [LightProver.fullRound_3_3, (sbox_uniqueAssignment _).equiv, (mds_3_uniqueAssignment _).equiv]; tauto

def halfRound_3_3_uniqueAssignment (S C : List.Vector F 3): UniqueAssignment (LightProver.halfRound_3_3 S C) id := UniqueAssignment.mk _ $ by
  simp [LightProver.halfRound_3_3, (sbox_uniqueAssignment _).equiv, (mds_3_uniqueAssignment _).equiv]; tauto

def poseidon_3_uniqueAssignment (inp : List.Vector F 3): UniqueAssignment (LightProver.poseidon_3 inp) id := by
  unfold LightProver.poseidon_3
  repeat (
    apply UniqueAssignment.compose
    . (first | apply fullRound_3_3_uniqueAssignment | apply halfRound_3_3_uniqueAssignment)
    intro _
  )
  apply UniqueAssignment.constant

theorem poseidon_3_testVector : (poseidon_3_uniqueAssignment (vec![0,1,2])).val = vec![0x115cc0f5e7d690413df64c6b9662e9cf2a3617f2743245519e19607a4417189a, 0x0fca49b798923ab0239de1c9e7a4a9a2210312b6a2f616d18b5a87f9b628ae29, 0x0e7ae82e40091e63cbd4f16a6d16310b3729d4b6e138fcf54110e2867045a30c] :=
  by native_decide

def poseidon₂ : Hash F 2 := fun a => (poseidon_3_uniqueAssignment vec![0, a.get 0, a.get 1]).val.get 0

@[simp]
lemma Poseidon2_iff_uniqueAssignment (a b : F) (k : F -> Prop) : LightProver.Poseidon2 a b k ↔ k (poseidon₂ vec![a, b]) := by
    unfold LightProver.Poseidon2 poseidon₂
    apply Iff.of_eq
    rw [(poseidon_3_uniqueAssignment _).equiv]
    congr

def mds_4_uniqueAssignment (S : List.Vector F 4): UniqueAssignment (LightProver.mds_4 S) id := UniqueAssignment.mk _ $ by
  simp [LightProver.mds_4]; tauto

def fullRound_4_4_uniqueAssignment (S C : List.Vector F 4): UniqueAssignment (LightProver.fullRound_4_4 S C) id := UniqueAssignment.mk _ $ by
  simp [LightProver.fullRound_4_4, (sbox_uniqueAssignment _).equiv, (mds_4_uniqueAssignment _).equiv]; tauto

def halfRound_4_4_uniqueAssignment (S C : List.Vector F 4): UniqueAssignment (LightProver.halfRound_4_4 S C) id := UniqueAssignment.mk _ $ by
  simp [LightProver.halfRound_4_4, (sbox_uniqueAssignment _).equiv, (mds_4_uniqueAssignment _).equiv]; tauto

def poseidon_4_uniqueAssignment (inp : List.Vector F 4): UniqueAssignment (LightProver.poseidon_4 inp) id := by
  unfold LightProver.poseidon_4
  repeat (
    apply UniqueAssignment.compose
    . (first | apply fullRound_4_4_uniqueAssignment | apply halfRound_4_4_uniqueAssignment)
    intro _
  )
  apply UniqueAssignment.constant

def poseidon₃ : Hash F 3 := fun a => (poseidon_4_uniqueAssignment vec![0, a.get 0, a.get 1, a.get 2]).val.get 0

@[simp]
lemma Poseidon3_iff_uniqueAssignment (a b c : F) (k : F -> Prop) : LightProver.Poseidon3 a b c k ↔ k (poseidon₃ vec![a, b, c]) := by
    unfold LightProver.Poseidon3 poseidon₃
    apply Iff.of_eq
    rw [(poseidon_4_uniqueAssignment _).equiv]
    congr
