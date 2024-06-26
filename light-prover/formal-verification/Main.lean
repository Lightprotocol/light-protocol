import ProvenZk
import FormalVerification.Circuit

open LightProver (F Order)
variable [Fact (Nat.Prime Order)]

lemma x : (2:F)^248 = 452312848583266388373324160190187140051835877600158453279131187530910662656 := by rfl
lemma pow2_248_val : ((2:F)^248).val = 2^248 := by rfl

def ZMod.toInt (n : ZMod N): ℤ := n.val
lemma pow2_248_toInt : ((2:F)^248).toInt = 2^248 := by rfl


lemma ZMod.toInt_nonneg [NeZero N] {n : ZMod N}: 0 ≤ n.toInt := by
  rw [toInt]
  apply Int.ofNat_nonneg

lemma ZMod.toInt_lt [NeZero N] {n : ZMod N}: n.toInt < N := by
  rw [toInt]
  rw [Nat.cast_lt]
  apply ZMod.val_lt

lemma ZMod.toInt_neg [NeZero N] {n : ZMod N}: (-n).toInt = -(n.toInt) % N := by
  simp [toInt, neg_val]
  split
  . simp [*]
  . rw [Nat.cast_sub]
    . rw [←Int.add_emod_self_left]
      rw [Int.emod_eq_of_lt]
      . congr; simp
      . simp
        apply le_of_lt
        rw [ZMod.cast_eq_val, Int.ofNat_lt]
        apply ZMod.val_lt
      . simp
        rw [ZMod.cast_eq_val, ←Int.ofNat_zero, Int.ofNat_lt]
        apply Nat.zero_lt_of_ne_zero
        simp [*]
    . exact Nat.le_of_lt (ZMod.val_lt _)

lemma ZMod.toInt_add [NeZero N] {n m : ZMod N}: (n + m).toInt = (m.toInt + n.toInt) % N := by
  simp [toInt, val_add, add_comm]

lemma ZMod.toInt_sub [NeZero N] {n m : ZMod N}: (n - m).toInt = (n.toInt - m.toInt) % N := by
  simp [sub_eq_add_neg, toInt_add, toInt_neg, add_comm]

@[simp]
lemma Gates.to_binary_succ_of_snoc_zero {n A R} : Gates.to_binary A n.succ (R.snoc (0:F)) ↔ Gates.to_binary A n R := by
  rw [Gates.to_binary_iff_eq_Fin_ofBitsLE]
  rw [Vector.exists_succ_iff_exists_snoc]
  unfold Fin.ofBitsLE
  unfold Fin.ofBitsBE
  simp [Bool.toNat, Gates.to_binary_iff_eq_Fin_ofBitsLE]
  tauto

lemma Int.ofNat_pow {a b : ℕ} : (a^b : ℤ) = (OfNat.ofNat a)^b := by simp [OfNat.ofNat]

-- TODO: This can be done better – remove the zero assert in circuit!
theorem AssertIsLess_248_semantics {A B : F} : LightProver.AssertIsLess_248 A B ↔ (A + (2^248 - B)).toInt < 2^248 := by
  unfold LightProver.AssertIsLess_248
  simp (config := { singlePass := true }) only [Gates.eq, Vector.exists_succ_iff_exists_snoc]
  simp only [Vector.getElem_snoc_at_length]
  simp [Gates.add, Gates.to_binary_succ_of_snoc_zero]
  simp [Gates.to_binary_iff_eq_Fin_ofBitsLE]
  apply Iff.intro
  . rintro ⟨vs, v', ⟨_⟩, h⟩
    have := congrArg ZMod.toInt h
    simp at this
    rw [x, this, ZMod.toInt, ZMod.val_nat_cast]
    rw [Nat.mod_eq_of_lt]
    . apply Int.lt_of_toNat_lt
      rw [Int.toNat_ofNat, ←Int.ofNat_pow, Int.toNat_ofNat]
      apply Fin.prop
    . apply Nat.lt_trans (m:=2^248) <;> simp
  . intro h
    rw [ZMod.toInt, ←Int.ofNat_pow, Nat.cast_lt] at h
    let v := Fin.toBitsLE $ Fin.mk (A + (2^248 - B)).val h
    refine ⟨v.map Bool.toZMod, v, Eq.refl _, ?r⟩
    simp [x]

theorem negSucc_le_negSucc (m n : Nat) : Int.negSucc m ≤ Int.negSucc n ↔ n ≤ m := by
  rw [Int.le_def]
  apply Iff.intro
  conv => lhs; arg 1; whnf
  split
  . rename_i h; intro; rw [Nat.succ_sub_succ_eq_sub] at h; exact Nat.le_of_sub_eq_zero h
  . intro; contradiction

theorem emod_negSucc (m : Nat) (n : Int) :
  (Int.negSucc m) % n = Int.subNatNat (Int.natAbs n) (Nat.succ (m % Int.natAbs n)) := rfl

theorem Int.mod_one_below {a : ℤ} {mod : ℤ} (hp : mod > 0) : a < 0 → a ≥ -mod → a % mod = a + mod := by
  intro hlt hge
  have := Int.eq_negSucc_of_lt_zero hlt
  rcases this with ⟨b, ⟨_⟩⟩
  have := Int.eq_succ_of_zero_lt hp
  rcases this with ⟨m, ⟨_⟩⟩
  conv at hge => rhs; whnf
  rw [emod_negSucc]
  simp [natAbs]
  rw [Nat.mod_eq_of_lt]
  rfl
  simp [negSucc_le_negSucc] at hge
  exact Nat.lt_succ_of_le hge

theorem Int.mod_pos_below {a : ℤ} {mod : ℤ} : 0 ≤ a → a < mod → a % mod = a := by
  intros
  simp [Int.emod_eq_of_lt, *]

theorem AssertIsLess_bounds { A B : F} (A_range : A.toInt ≤ 2 ^ 249): LightProver.AssertIsLess_248 A B → A.toInt < B.toInt ∧ B.toInt ≤ A.toInt + 2^248 := by
  rw [AssertIsLess_248_semantics, ZMod.toInt_add, ZMod.toInt_sub]
  simp [pow2_248_toInt]
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

theorem AssertIsLess_range {hi lo val : F} {lo_range : lo.toInt < 2^248} :
  LightProver.AssertIsLess_248 lo val ∧ LightProver.AssertIsLess_248 val hi → lo.toInt < val.toInt ∧ val.toInt < hi.toInt := by
  rintro ⟨hlo, hhi⟩
  have ⟨hl, nextRange⟩ := AssertIsLess_bounds (by linarith) hlo
  have val_range : val.toInt ≤ 2^249 := by linarith
  have ⟨hv, _⟩ := AssertIsLess_bounds val_range hhi
  exact ⟨hl, hv⟩
