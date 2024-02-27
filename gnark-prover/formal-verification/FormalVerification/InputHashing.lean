import FormalVerification
import FormalVerification.Common
import FormalVerification.ReducednessCheck
import FormalVerification.BinaryDecompositions
import FormalVerification.Keccak
import ProvenZk
import Mathlib.Data.Vector.MapLemmas
open SemaphoreMTB (F Order)

lemma ZMod.eq_iff_veq {N : ℕ} {a b : ZMod (N.succ)} : a = b ↔ a.val = b.val := by
  apply Iff.intro
  . intro h; subst h; rfl
  . intro hp
    simp [ZMod.val] at hp
    apply Fin.eq_of_veq hp

def RC : Vector (Fin (2 ^ 64)) 24 := vec![0x0000000000000001, 0x0000000000008082, 0x800000000000808A, 0x8000000080008000, 0x000000000000808B, 0x0000000080000001, 0x8000000080008081, 0x8000000000008009, 0x000000000000008A, 0x0000000000000088, 0x0000000080008009, 0x000000008000000A, 0x000000008000808B, 0x800000000000008B, 0x8000000000008089, 0x8000000000008003, 0x8000000000008002, 0x8000000000000080, 0x000000000000800A, 0x800000008000000A, 0x8000000080008081, 0x8000000000008080, 0x0000000080000001, 0x8000000080008008]

def RCBits : Vector (Vector Bool 64) 24 := RC.map Fin.toBitsLE

def RCBitsField : Vector (Vector F 64) 24 := vec![vec![(1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F)], vec![(0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F)], vec![(0:F), (1:F), (0:F), (1:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F)], vec![(0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F)], vec![(1:F), (1:F), (0:F), (1:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F)], vec![(1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F)], vec![(1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F)], vec![(1:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F)], vec![(0:F), (1:F), (0:F), (1:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F)], vec![(0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F)], vec![(1:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F)], vec![(0:F), (1:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F)], vec![(1:F), (1:F), (0:F), (1:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F)], vec![(1:F), (1:F), (0:F), (1:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F)], vec![(1:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F)], vec![(1:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F)], vec![(0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F)], vec![(0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F)], vec![(0:F), (1:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F)], vec![(0:F), (1:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F)], vec![(1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F)], vec![(0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F)], vec![(1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F)], vec![(0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (0:F), (1:F)]]

theorem RCBitsField_def : RCBitsField = RCBits.map (Vector.map Bool.toZMod) := by native_decide

def DeletionMbuCircuit_4_4_30_4_4_30_Fold (InputHash: F) (DeletionIndices: Vector F 4) (PreRoot: F) (PostRoot: F) (IdComms: Vector F 4) (MerkleProofs: Vector (Vector F 30) 4): Prop :=
  SemaphoreMTB.ToReducedBigEndian_32 DeletionIndices[0] fun gate_0 =>
  SemaphoreMTB.ToReducedBigEndian_32 DeletionIndices[1] fun gate_1 =>
  SemaphoreMTB.ToReducedBigEndian_32 DeletionIndices[2] fun gate_2 =>
  SemaphoreMTB.ToReducedBigEndian_32 DeletionIndices[3] fun gate_3 =>
  SemaphoreMTB.ToReducedBigEndian_256 PreRoot fun gate_4 =>
  SemaphoreMTB.ToReducedBigEndian_256 PostRoot fun gate_5 =>
  SemaphoreMTB.KeccakGadget_640_64_24_640_256_24_1088_1
    (Vector.ofFnGet gate_0 ++ Vector.ofFnGet gate_1 ++ Vector.ofFnGet gate_2 ++ Vector.ofFnGet gate_3 ++ Vector.ofFnGet gate_4 ++ Vector.ofFnGet gate_5) RCBitsField fun gate_6 =>
  SemaphoreMTB.FromBinaryBigEndian_256 gate_6 fun gate_7 =>
  Gates.eq InputHash gate_7 ∧
  SemaphoreMTB.DeletionProof_4_4_30_4_4_30 DeletionIndices PreRoot IdComms MerkleProofs fun gate_9 =>
  Gates.eq gate_9 PostRoot ∧
  True

theorem DeletionCircuit_folded {InputHash PreRoot PostRoot : F} {DeletionIndices IdComms : Vector F 4} {MerkleProofs: Vector (Vector F 30) 4}:
  SemaphoreMTB.DeletionMbuCircuit_4_4_30_4_4_30 InputHash DeletionIndices PreRoot PostRoot IdComms MerkleProofs =
  DeletionMbuCircuit_4_4_30_4_4_30_Fold InputHash DeletionIndices PreRoot PostRoot IdComms MerkleProofs := by rfl

lemma Vector.map_hAppend {n₁ n₂ α β} {v₁ : Vector α n₁} {v₂ : Vector α n₂} {f : α → β}: Vector.map f v₁ ++ Vector.map f v₂ = Vector.map f (v₁ ++ v₂) := by
  apply Vector.eq
  simp

theorem Deletion_InputHash_deterministic :
    SemaphoreMTB.DeletionMbuCircuit_4_4_30_4_4_30 InputHash₁ DeletionIndices PreRoot PostRoot IdComms₁ MerkleProofs₁ ∧
    SemaphoreMTB.DeletionMbuCircuit_4_4_30_4_4_30 InputHash₂ DeletionIndices PreRoot PostRoot IdComms₂ MerkleProofs₂ →
    InputHash₁ = InputHash₂ := by
  intro ⟨h₁, h₂⟩
  rw [DeletionCircuit_folded] at h₁ h₂
  unfold DeletionMbuCircuit_4_4_30_4_4_30_Fold at h₁ h₂
  simp only [
    Vector.ofFnGet_id,
    ToReducedBigEndian_32_uncps,
    ToReducedBigEndian_256_uncps,
    RCBitsField_def,
    ←Vector.map_permute,
    Vector.map_hAppend,
    (KeccakGadget_640_64_24_640_256_24_1088_1_uniqueAssignment _ _).equiv,
    FromBinaryBigEndian_256_uncps,
    Gates.eq
  ] at h₁ h₂
  rcases h₁ with ⟨_, _, _, _, h₁, _⟩
  rcases h₂ with ⟨_, _, _, _, h₂, _⟩
  simp [h₁, h₂]

theorem Deletion_skipHashing :
  SemaphoreMTB.DeletionMbuCircuit_4_4_30_4_4_30 InputHash DeletionIndices PreRoot PostRoot IdComms MerkleProofs →
  SemaphoreMTB.DeletionProof_4_4_30_4_4_30 DeletionIndices PreRoot IdComms MerkleProofs fun res => res = PostRoot := by
  repeat rw [DeletionCircuit_folded]
  unfold DeletionMbuCircuit_4_4_30_4_4_30_Fold
  simp only [
    Vector.ofFnGet_id,
    ToReducedBigEndian_32_uncps,
    ToReducedBigEndian_256_uncps,
    RCBitsField_def,
    ←Vector.map_permute,
    Vector.map_hAppend,
    (KeccakGadget_640_64_24_640_256_24_1088_1_uniqueAssignment _ _).equiv,
    FromBinaryBigEndian_256_uncps,
    Gates.eq
  ]
  simp

def reducedKeccak640 (v : Vector Bool 640) : F :=
  (Fin.ofBitsLE (Vector.permute rev_ix_256 (KeccakGadget_640_64_24_640_256_24_1088_1_uniqueAssignment v RCBits).val)).val

theorem reducedKeccak640_zeros :
  reducedKeccak640 (Vector.replicate 640 false) = 4544803827027086362579185658884299814463816764684804205918064517636252260498 := by
  native_decide

theorem reducedKeccak640_ones :
  reducedKeccak640 (Vector.replicate 640 true) = 1953461151768174703550518710286555794214949287664819893310466469381641334512 := by
  native_decide

theorem Deletion_InputHash_injective :
  Function.Injective reducedKeccak640 →
  SemaphoreMTB.DeletionMbuCircuit_4_4_30_4_4_30 InputHash DeletionIndices₁ PreRoot₁ PostRoot₁ IdComms₁ MerkleProofs₁ ∧
  SemaphoreMTB.DeletionMbuCircuit_4_4_30_4_4_30 InputHash DeletionIndices₂ PreRoot₂ PostRoot₂ IdComms₂ MerkleProofs₂ →
  DeletionIndices₁ = DeletionIndices₂ ∧ PreRoot₁ = PreRoot₂ ∧ PostRoot₁ = PostRoot₂ := by
  intro kr ⟨h₁, h₂⟩
  rw [DeletionCircuit_folded] at h₁ h₂
  unfold DeletionMbuCircuit_4_4_30_4_4_30_Fold at h₁ h₂
  simp only [
    Vector.ofFnGet_id,
    ToReducedBigEndian_32_uncps,
    ToReducedBigEndian_256_uncps,
    RCBitsField_def,
    ←Vector.map_permute,
    Vector.map_hAppend,
    (KeccakGadget_640_64_24_640_256_24_1088_1_uniqueAssignment _ _).equiv,
    FromBinaryBigEndian_256_uncps,
    Gates.eq
  ] at h₁ h₂
  rcases h₁ with ⟨_, _, _, _, h₁, _⟩
  rcases h₂ with ⟨_, _, _, _, h₂, _⟩
  rw [h₁] at h₂
  replace h₂ := kr h₂
  repeat rw [Vector.append_inj_iff] at h₂
  repeat rw [Function.Injective.eq_iff (Vector.permute_inj rev_ix_256_surj)] at h₂
  repeat rw [Function.Injective.eq_iff (Vector.permute_inj rev_ix_32_surj)] at h₂
  repeat rw [Fin.toBitsLE_injective] at h₂
  repeat rw [Fin.eq_iff_veq] at h₂
  simp [←ZMod.eq_iff_veq, getElem] at h₂
  casesm* _ ∧ _
  refine ⟨?_, by assumption, by assumption⟩
  ext i
  fin_cases i <;> simp [*]

def InsertionMbuCircuit_4_30_4_4_30_Fold (InputHash: F) (StartIndex: F) (PreRoot: F) (PostRoot: F) (IdComms: Vector F 4) (MerkleProofs: Vector (Vector F 30) 4): Prop :=
    SemaphoreMTB.ToReducedBigEndian_32 StartIndex fun gate_0 =>
    SemaphoreMTB.ToReducedBigEndian_256 PreRoot fun gate_1 =>
    SemaphoreMTB.ToReducedBigEndian_256 PostRoot fun gate_2 =>
    SemaphoreMTB.ToReducedBigEndian_256 IdComms[0] fun gate_3 =>
    SemaphoreMTB.ToReducedBigEndian_256 IdComms[1] fun gate_4 =>
    SemaphoreMTB.ToReducedBigEndian_256 IdComms[2] fun gate_5 =>
    SemaphoreMTB.ToReducedBigEndian_256 IdComms[3] fun gate_6 =>
    SemaphoreMTB.KeccakGadget_1568_64_24_1568_256_24_1088_1
        (Vector.ofFnGet gate_0 ++ Vector.ofFnGet gate_1 ++ Vector.ofFnGet gate_2 ++ Vector.ofFnGet gate_3 ++ Vector.ofFnGet gate_4 ++ Vector.ofFnGet gate_5 ++ Vector.ofFnGet gate_6) RCBitsField fun gate_7 =>
    SemaphoreMTB.FromBinaryBigEndian_256 gate_7 fun gate_8 =>
    Gates.eq InputHash gate_8 ∧
    SemaphoreMTB.InsertionProof_4_30_4_4_30 StartIndex PreRoot IdComms MerkleProofs fun gate_10 =>
    Gates.eq gate_10 PostRoot ∧
    True

theorem InsertionMbuCircuit_4_30_4_4_30_folded:
  SemaphoreMTB.InsertionMbuCircuit_4_30_4_4_30 InputHash StartIndex PreRoot PostRoot IdComms MerkleProofs =
  InsertionMbuCircuit_4_30_4_4_30_Fold InputHash StartIndex PreRoot PostRoot IdComms MerkleProofs := by rfl

theorem Insertion_InputHash_deterministic :
  SemaphoreMTB.InsertionMbuCircuit_4_30_4_4_30 InputHash₁ StartIndex PreRoot PostRoot IdComms MerkleProofs₁ ∧
  SemaphoreMTB.InsertionMbuCircuit_4_30_4_4_30 InputHash₂ StartIndex PreRoot PostRoot IdComms MerkleProofs₂ →
  InputHash₁ = InputHash₂ := by
  intro ⟨h₁, h₂⟩
  rw [InsertionMbuCircuit_4_30_4_4_30_folded] at h₁ h₂
  unfold InsertionMbuCircuit_4_30_4_4_30_Fold at h₁ h₂
  simp only [
    Vector.ofFnGet_id,
    ToReducedBigEndian_32_uncps,
    ToReducedBigEndian_256_uncps,
    RCBitsField_def,
    ←Vector.map_permute,
    Vector.map_hAppend,
    (KeccakGadget_1568_64_24_1568_256_24_1088_1_uniqueAssignment _ _).equiv,
    FromBinaryBigEndian_256_uncps,
    Gates.eq
  ] at h₁ h₂
  rcases h₁ with ⟨_, h₁, _⟩
  rcases h₂ with ⟨_, h₂, _⟩
  simp [h₁, h₂]

def reducedKeccak1568 (v : Vector Bool 1568) : F :=
  (Fin.ofBitsLE (Vector.permute rev_ix_256 (KeccakGadget_1568_64_24_1568_256_24_1088_1_uniqueAssignment v RCBits).val)).val

theorem reducedKeccak1568_zeros :
  reducedKeccak1568 (Vector.replicate 1568 false) = 0x2872693cd1edb903471cf4a03c1e436f32dccf7ffa2218a4e0354c2514004511 := by
  native_decide

theorem reducedKeccak1568_ones :
  reducedKeccak1568 (Vector.replicate 1568 true) = 0x1d7add23b253ac47705200179f6ea5df39ba965ccda0a213c2afc112bc842a5 := by
  native_decide

theorem Insertion_InputHash_injective :
  Function.Injective reducedKeccak1568 →
  SemaphoreMTB.InsertionMbuCircuit_4_30_4_4_30 InputHash StartIndex₁ PreRoot₁ PostRoot₁ IdComms₁ MerkleProofs₁ ∧
  SemaphoreMTB.InsertionMbuCircuit_4_30_4_4_30 InputHash StartIndex₂ PreRoot₂ PostRoot₂ IdComms₂ MerkleProofs₂ →
  StartIndex₁ = StartIndex₂ ∧ PreRoot₁ = PreRoot₂ ∧ PostRoot₁ = PostRoot₂ ∧ IdComms₁ = IdComms₂ := by
  intro kr ⟨h₁, h₂⟩
  rw [InsertionMbuCircuit_4_30_4_4_30_folded] at h₁ h₂
  unfold InsertionMbuCircuit_4_30_4_4_30_Fold at h₁ h₂
  simp only [
    Vector.ofFnGet_id,
    ToReducedBigEndian_32_uncps,
    ToReducedBigEndian_256_uncps,
    RCBitsField_def,
    ←Vector.map_permute,
    Vector.map_hAppend,
    (KeccakGadget_1568_64_24_1568_256_24_1088_1_uniqueAssignment _ _).equiv,
    FromBinaryBigEndian_256_uncps,
    Gates.eq
  ] at h₁ h₂
  rcases h₁ with ⟨_, h₁, _⟩
  rcases h₂ with ⟨_, h₂, _⟩
  rw [h₁] at h₂
  replace h₂ := kr h₂
  repeat rw [Vector.append_inj_iff] at h₂
  repeat rw [Function.Injective.eq_iff (Vector.permute_inj rev_ix_256_surj)] at h₂
  repeat rw [Function.Injective.eq_iff (Vector.permute_inj rev_ix_32_surj)] at h₂
  repeat rw [Fin.toBitsLE_injective] at h₂
  repeat rw [Fin.eq_iff_veq] at h₂
  simp [←ZMod.eq_iff_veq, and_assoc, getElem] at h₂
  casesm* _ ∧ _
  refine ⟨by assumption, by assumption, by assumption, ?_⟩
  ext i
  fin_cases i <;> simp [*]

theorem Insertion_skipHashing :
  SemaphoreMTB.InsertionMbuCircuit_4_30_4_4_30 InputHash StartIndex PreRoot PostRoot IdComms MerkleProofs →
  SemaphoreMTB.InsertionProof_4_30_4_4_30 StartIndex PreRoot IdComms MerkleProofs fun res => res = PostRoot := by
  intro h
  rw [InsertionMbuCircuit_4_30_4_4_30_folded] at h
  unfold InsertionMbuCircuit_4_30_4_4_30_Fold at h
  simp only [
    Vector.ofFnGet_id,
    ToReducedBigEndian_32_uncps,
    ToReducedBigEndian_256_uncps,
    RCBitsField_def,
    ←Vector.map_permute,
    Vector.map_hAppend,
    (KeccakGadget_1568_64_24_1568_256_24_1088_1_uniqueAssignment _ _).equiv,
    FromBinaryBigEndian_256_uncps,
    Gates.eq
  ] at h
  rcases h with ⟨_, _, h⟩
  simp at h
  exact h
