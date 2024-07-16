import ProvenZk
import FormalVerification.Circuit
import FormalVerification.Lemmas

open LightProver (F Order)
variable [Fact (Nat.Prime Order)]

theorem AssertIsLess_248_semantics {A B : F} : LightProver.AssertIsLess_248 A B ↔ (A + (2^248 - B)).toInt < 2^248 := by
  unfold LightProver.AssertIsLess_248
  simp [LightProver.AssertIsLess_248, Gates.add]
  apply Iff.intro
  . rintro ⟨_, hp⟩
    have hp := Gates.to_binary_rangecheck hp
    apply Int.lt_of_toNat_lt
    exact hp
  . intro hp
    unfold ZMod.toInt at hp
    have : (2:ℤ)^248 = Nat.cast 452312848583266388373324160190187140051835877600158453279131187530910662656 := by rfl
    rw [this, Int.ofNat_lt] at hp
    simp_rw [Gates.to_binary_iff_eq_Fin_ofBitsLE]
    rw [exists_swap]
    exists Fin.toBitsLE (Fin.mk (A + (2^248 - B)).val hp)
    simp; rfl

theorem AssertIsLess_bounds { A B : F} (A_range : A.toInt ≤ 2 ^ 249): LightProver.AssertIsLess_248 A B → A.toInt < B.toInt ∧ B.toInt ≤ A.toInt + 2^248 := by
  rw [AssertIsLess_248_semantics, ZMod.toInt_add, ZMod.toInt_sub]
  have : ((2:F)^248).toInt = 2^248 := by rfl
  simp [this]
  have hge : (2^248 - B.toInt + A.toInt) ≥ -Order := by
    . linarith [ZMod.toInt_nonneg (n:=A), ZMod.toInt_lt (n:=B)]
  have hle : (2^248 - B.toInt + A.toInt) < Order := by
      have : A.toInt + 2^248 < Order := by
        calc
          A.toInt + (2:ℤ)^248 ≤ ((2:ℤ)^249 + (2:ℤ)^248 : ℤ) := by linarith
          _ < Order := by decide
      linarith [ZMod.toInt_nonneg (n:=B)]
  cases lt_or_ge (2^248 - B.toInt + A.toInt) 0 with
  | inl h =>
    rw [Int.mod_one_below (by decide) h hge]
    intro hp
    linarith [ZMod.toInt_lt (n:=B), ZMod.toInt_nonneg (n:=A)]
  | inr h =>
    rw [Int.mod_pos_below h hle]
    intro hp
    apply And.intro
    . linarith
    . linarith

theorem AssertIsLess_range {hi lo val : F} (lo_range : lo.toInt < 2^248) :
  LightProver.AssertIsLess_248 lo val ∧ LightProver.AssertIsLess_248 val hi → lo.toInt < val.toInt ∧ val.toInt < hi.toInt := by
  rintro ⟨hlo, hhi⟩
  have ⟨hl, nextRange⟩ := AssertIsLess_bounds (by linarith) hlo
  have val_range : val.toInt ≤ 2^249 := by linarith
  have ⟨hv, _⟩ := AssertIsLess_bounds val_range hhi
  exact ⟨hl, hv⟩
