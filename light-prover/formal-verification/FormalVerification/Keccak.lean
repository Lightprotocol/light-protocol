import FormalVerification
import FormalVerification.Common
import ProvenZk
import Mathlib

open SemaphoreMTB (F Order)

def Xor_64_64_uniqueAssignment (v1 v2 : Vector Bool 64):
  UniqueAssignment (SemaphoreMTB.Xor_64_64 (v1.map Bool.toZMod) (v2.map Bool.toZMod)) (Vector.map Bool.toZMod) := by
  simp [SemaphoreMTB.Xor_64_64, Vector.getElem_map, Gates.xor_bool]
  rw [←Vector.map_nil]
  repeat rw [←Vector.map_cons]
  apply UniqueAssignment.constant

def And_64_64_uniqueAssignment (v1 v2 : Vector Bool 64):
  UniqueAssignment (SemaphoreMTB.And_64_64 (v1.map Bool.toZMod) (v2.map Bool.toZMod)) (Vector.map Bool.toZMod) := by
  simp [SemaphoreMTB.And_64_64, Vector.getElem_map, Gates.and_bool]
  rw [←Vector.map_nil]
  repeat rw [←Vector.map_cons]
  apply UniqueAssignment.constant

def Not_64_uniqueAssignment (v1 : Vector Bool 64):
  UniqueAssignment (SemaphoreMTB.Not_64 (v1.map Bool.toZMod)) (Vector.map Bool.toZMod) := by
  simp [SemaphoreMTB.Not_64, Vector.getElem_map, Gates.not_bool]
  rw [←Vector.map_nil]
  repeat rw [←Vector.map_cons]
  apply UniqueAssignment.constant

def Xor5Round_uniqueAssignment {v1 v2 v3 v4 v5 : Bool}:
  UniqueAssignment (SemaphoreMTB.Xor5Round v1.toZMod v2.toZMod v3.toZMod v4.toZMod v5.toZMod) Bool.toZMod := by
  simp [SemaphoreMTB.Xor5Round, Gates.xor_bool]
  apply UniqueAssignment.constant

def Xor5_64_64_64_64_64_uniqueAssignment {v1 v2 v3 v4 v5 : Vector Bool 64}:
  UniqueAssignment (SemaphoreMTB.Xor5_64_64_64_64_64 (v1.map Bool.toZMod) (v2.map Bool.toZMod) (v3.map Bool.toZMod) (v4.map Bool.toZMod) (v5.map Bool.toZMod)) (Vector.map Bool.toZMod) := by
  unfold SemaphoreMTB.Xor5_64_64_64_64_64
  simp only [Vector.getElem_map]
  repeat (
    apply UniqueAssignment.compose
    apply Xor5Round_uniqueAssignment
    intro _
  )
  rw [←Vector.map_nil]
  repeat rw [←Vector.map_cons]
  apply UniqueAssignment.constant

def KeccakRound_64_5_5_64_uniqueAssignment
  { state : Vector (Vector (Vector Bool 64) 5) 5}
  { rc : Vector Bool 64 }:
  UniqueAssignment (SemaphoreMTB.KeccakRound_64_5_5_64 (Vector.map (Vector.map (Vector.map Bool.toZMod)) state) (rc.map Bool.toZMod)) (Vector.map (Vector.map (Vector.map Bool.toZMod))) := by
  unfold SemaphoreMTB.KeccakRound_64_5_5_64
  simp only [Vector.getElem_map]

  repeat (
    apply UniqueAssignment.compose
    apply Xor5_64_64_64_64_64_uniqueAssignment
    intro _
  )

  repeat (
    apply UniqueAssignment.compose (embf := Vector.map Bool.toZMod)
    . unfold SemaphoreMTB.Rot_64_1
      simp only [Vector.getElem_map]
      rw [←Vector.map_nil]
      repeat rw [←Vector.map_cons]
      apply UniqueAssignment.constant
    intro _
    apply UniqueAssignment.compose
    apply Xor_64_64_uniqueAssignment
    intro _
  )

  repeat (
    apply UniqueAssignment.compose
    apply Xor_64_64_uniqueAssignment
    intro _
  )

  (
    apply UniqueAssignment.compose (embf := Vector.map Bool.toZMod)
    . apply UniqueAssignment.constant
    intro _
  )

  repeat (
    apply UniqueAssignment.compose (embf := Vector.map Bool.toZMod)
    . apply UniqueAssignment.constant'
      simp only [Vector.getElem_map]
      rw [←Vector.map_nil]
      repeat rw [←Vector.map_cons]
    intro _
  )

  repeat (
    apply UniqueAssignment.compose
    apply Not_64_uniqueAssignment
    intro _
    apply UniqueAssignment.compose
    apply And_64_64_uniqueAssignment
    intro _
    apply UniqueAssignment.compose
    apply Xor_64_64_uniqueAssignment
    intro _
  )

  apply UniqueAssignment.compose
  apply Xor_64_64_uniqueAssignment
  intro _

  apply UniqueAssignment.constant'
  repeat rw [←Vector.map_singleton (f := Vector.map Bool.toZMod)]
  repeat rw [←Vector.map_cons]
  rw [←Vector.map_singleton]
  repeat rw [←Vector.map_cons]

def KeccakF_64_5_5_64_24_24_uniqueAssignment
  { state : Vector (Vector (Vector Bool 64) 5) 5}
  { rc : Vector (Vector Bool 64) 24 }:
  UniqueAssignment (SemaphoreMTB.KeccakF_64_5_5_64_24_24 (Vector.map (Vector.map (Vector.map Bool.toZMod)) state) (rc.map (Vector.map Bool.toZMod))) (Vector.map (Vector.map (Vector.map Bool.toZMod))) := by
  unfold SemaphoreMTB.KeccakF_64_5_5_64_24_24
  repeat (
    apply UniqueAssignment.compose
    . simp only [Vector.getElem_map]
      apply KeccakRound_64_5_5_64_uniqueAssignment
    intro _
  )
  apply UniqueAssignment.constant

def KeccakGadget_640_64_24_640_256_24_1088_1_uniqueAssignment
  (input : Vector Bool 640)
  ( rc : Vector (Vector Bool 64) 24):
  UniqueAssignment (SemaphoreMTB.KeccakGadget_640_64_24_640_256_24_1088_1 (input.map Bool.toZMod) (rc.map (Vector.map Bool.toZMod))) (Vector.map Bool.toZMod) := by
  unfold SemaphoreMTB.KeccakGadget_640_64_24_640_256_24_1088_1
  simp only [ ←Bool.toZMod_zero
            , ←Bool.toZMod_one
            , Vector.getElem_map
            ]
  simp only [Gates.xor_bool, exists_eq_left]
  simp only [ ←Vector.map_singleton (f:=Bool.toZMod)
            , ←Vector.map_singleton (f:=Vector.map Bool.toZMod)
            , ←Vector.map_singleton (f:=Vector.map (Vector.map Bool.toZMod))
            , ←Vector.map_cons
            ]
  apply UniqueAssignment.compose
  apply KeccakF_64_5_5_64_24_24_uniqueAssignment
  intro _
  simp only [Vector.getElem_map]
  simp only [ ←Vector.map_singleton (f:=Bool.toZMod)
            , ←Vector.map_cons
            ]
  apply UniqueAssignment.constant

def KeccakGadget_1568_64_24_1568_256_24_1088_1_uniqueAssignment
  (input : Vector Bool 1568)
  ( rc : Vector (Vector Bool 64) 24):
  UniqueAssignment (SemaphoreMTB.KeccakGadget_1568_64_24_1568_256_24_1088_1 (input.map Bool.toZMod) (rc.map (Vector.map Bool.toZMod))) (Vector.map Bool.toZMod) := by
  unfold SemaphoreMTB.KeccakGadget_1568_64_24_1568_256_24_1088_1
  simp only [ ←Bool.toZMod_zero
            , ←Bool.toZMod_one
            , Vector.getElem_map
            ]
  simp only [Gates.xor_bool, exists_eq_left]
  simp only [ ←Vector.map_singleton (f:=Bool.toZMod)
            , ←Vector.map_singleton (f:=Vector.map Bool.toZMod)
            , ←Vector.map_singleton (f:=Vector.map (Vector.map Bool.toZMod))
            , ←Vector.map_cons
            ]
  apply UniqueAssignment.compose
  apply KeccakF_64_5_5_64_24_24_uniqueAssignment
  intro _
  simp only [Vector.getElem_map]
  repeat (
    apply UniqueAssignment.compose
    . apply Xor_64_64_uniqueAssignment
    intro _
  )
  simp only [ ←Vector.map_singleton (f:=Vector.map (Vector.map Bool.toZMod))
            , ←Vector.map_singleton (f:=Vector.map Bool.toZMod)
            , ←Vector.map_cons
            ]
  apply UniqueAssignment.compose
  apply KeccakF_64_5_5_64_24_24_uniqueAssignment
  intro _

  simp only [Vector.getElem_map]
  simp only [ ←Vector.map_singleton (f:=Bool.toZMod)
            , ←Vector.map_cons
            ]
  apply UniqueAssignment.constant
