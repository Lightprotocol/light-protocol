import FormalVerification
import FormalVerification.Common
import FormalVerification.ReducednessCheck
import ProvenZk

open SemaphoreMTB (F Order)

lemma ReducedToBinary_256_iff_Fin_toBitsLE {f : F} {v : Vector F 256}:
  Gates.to_binary f 256 v ∧ SemaphoreMTB.ReducedModRCheck_256 v ↔
  v = (Fin.toBitsLE ⟨f.val, Nat.lt_trans (Fin.is_lt f) (by decide)⟩).map Bool.toZMod := by
  rw [Gates.to_binary_iff_eq_Fin_ofBitsLE]
  apply Iff.intro
  . intro ⟨bin, red⟩
    rcases bin with ⟨v, a, b⟩
    subst_vars
    rw [ReducedModRCheck_256_semantics] at red
    have {h} : Fin.mk (ZMod.val ((Fin.ofBitsLE v).val : F)) h = Fin.ofBitsLE v := by
      simp [ZMod.val_cast_of_lt, red]
    simp [this]
  . rintro ⟨_⟩
    simp [ReducedModRCheck_256_semantics, ZMod.val_lt]

def rev_ix_256 (i : Fin 256): Fin 256 := 248 - (i / 8) * 8 + i % 8
def rev_ix_32 (i : Fin 32): Fin 32 := 24 - (i / 8) * 8 + i % 8

theorem rev_ix_256_surj : Function.Surjective rev_ix_256 := by
  intro i
  exists rev_ix_256 i
  revert i
  decide

theorem rev_ix_32_surj : Function.Surjective rev_ix_32 := by
  intro i
  exists rev_ix_32 i
  revert i
  decide

theorem ToReducedBigEndian_256_uncps {f k}:
  SemaphoreMTB.ToReducedBigEndian_256 f k ↔ k (Vector.permute rev_ix_256 ((Fin.toBitsLE ⟨f.val, Nat.lt_trans (Fin.is_lt f) (by decide)⟩).map Bool.toZMod)) := by
  unfold SemaphoreMTB.ToReducedBigEndian_256
  conv =>
    pattern _ ::ᵥ _
    change Vector.permute rev_ix_256 gate_0
  apply Iff.intro
  . intro ⟨g, a, b, c⟩
    rw [ReducedToBinary_256_iff_Fin_toBitsLE.mp ⟨a, b⟩] at c
    assumption
  . intro _
    simp_rw [←and_assoc, ReducedToBinary_256_iff_Fin_toBitsLE]
    simp [*]

theorem ToReducedBigEndian_32_uncps {f k}:
  SemaphoreMTB.ToReducedBigEndian_32 f k ↔ ∃(h : f.val < 2^32), k (Vector.permute rev_ix_32 ((Fin.toBitsLE ⟨f.val, h⟩).map Bool.toZMod)) := by
  unfold SemaphoreMTB.ToReducedBigEndian_32
  unfold SemaphoreMTB.ReducedModRCheck_32
  conv =>
    pattern _ ::ᵥ _
    change Vector.permute rev_ix_32 gate_0
  simp_rw [Gates.to_binary_iff_eq_fin_to_bits_le_of_pow_length_lt]
  apply Iff.intro
  . rintro ⟨_, ⟨_, ⟨_⟩⟩, _, cont⟩
    simp [*]
  . rintro ⟨_, _⟩
    simp [*]

theorem FromBinaryBigEndian_256_uncps {f k}:
  SemaphoreMTB.FromBinaryBigEndian_256 (Vector.map Bool.toZMod f) k ↔ k (Fin.ofBitsLE (Vector.permute rev_ix_256 f)).val := by
  unfold SemaphoreMTB.FromBinaryBigEndian_256
  conv =>
    pattern _ ::ᵥ _
    change Vector.permute rev_ix_256 (f.map Bool.toZMod)
  simp [←Vector.map_permute, Gates.from_binary_iff_eq_ofBitsLE_mod_order]
