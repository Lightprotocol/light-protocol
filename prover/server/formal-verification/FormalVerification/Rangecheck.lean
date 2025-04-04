import «ProvenZk»
import FormalVerification.Circuit
import FormalVerification.Lemmas

open LightProver (F Order Gates)

@[simp]
lemma two_pow_248_zmod_val : (452312848583266388373324160190187140051835877600158453279131187530910662656 : ZMod Order) =  2^248 := by native_decide

@[simp]
lemma two_pow_248_z_val : (452312848583266388373324160190187140051835877600158453279131187530910662656 : ℤ) = 2^248 := by native_decide

theorem AssertIsLess_248_semantics {A B : F} : LightProver.AssertIsLess_248 A B ↔ (A + (2^248 - B)).val < 2^248 := by
  unfold LightProver.AssertIsLess_248
  simp [LightProver.AssertIsLess_248, Gates_base.add]
  apply Iff.intro
  . rintro ⟨_, hp⟩
    cases hp
    assumption
  . intro hp
    simp [Gates, GatesGnark12, GatesDef.to_binary_12, GatesGnark9, GatesGnark8, GatesDef.add, hp]

example : LightProver.AssertIsLess_248 (Order - 20) 10 ∧ (Order - 20 : F).val > 10 := by
  rw [AssertIsLess_248_semantics]; native_decide

set_option maxRecDepth 10000
theorem AssertIsLess_bounds { A B : F} (A_range : A.val ≤ 2 ^ 249): LightProver.AssertIsLess_248 A B → A.val < B.val ∧ B.val ≤ A.val + 2^248 := by
  rw [AssertIsLess_248_semantics];
  zify; simp;
  zify at A_range; simp at A_range;
  simp [ZMod.castInt_add, ZMod.castInt_sub]
  have : (((2:F)^248).cast:ℤ) = 2^248 := by rfl
  simp [this]
  have hge : (A.cast:ℤ) + (2^248 - (B.cast:ℤ)) ≥ -Order := by
    linarith [ZMod.castInt_nonneg (n:=A), ZMod.castInt_lt (n:=B)]
  have hle : (A.cast:ℤ) + (2^248 - (B.cast:ℤ)) < Order := by
      have : (A.cast:ℤ) + 2^248 < Order := by
        calc
          (A.cast:ℤ) + (2:ℤ)^248 ≤ ((2:ℤ)^249 + (2:ℤ)^248 : ℤ) := by linarith
          _ < Order := by decide
      linarith [ZMod.castInt_nonneg (n:=B)]
  cases lt_or_ge ((A.cast:ℤ) + (2^248 - (B.cast:ℤ))) 0 with
  | inl h =>
    rw [two_pow_248_z_val]
    rw [Int.emod_eq_add_self_of_neg_and_lt_neg_self h hge]
    intro hp
    linarith [ZMod.castInt_lt (n:=B), ZMod.castInt_nonneg (n:=A)]
  | inr h =>
    rw [two_pow_248_z_val]
    rw [Int.emod_eq_of_lt h hle]
    intro hp
    apply And.intro
    . linarith
    . linarith

theorem AssertIsLess_range {hi lo val : F} (lo_range : lo.val < 2^248) :
  LightProver.AssertIsLess_248 lo val ∧ LightProver.AssertIsLess_248 val hi → lo.val < val.val ∧ val.val < hi.val := by
  rintro ⟨hlo, hhi⟩
  have ⟨hl, nextRange⟩ := AssertIsLess_bounds (by linarith) hlo
  have val_range : val.val ≤ 2^249 := by linarith
  have ⟨hv, _⟩ := AssertIsLess_bounds val_range hhi
  exact ⟨hl, hv⟩
