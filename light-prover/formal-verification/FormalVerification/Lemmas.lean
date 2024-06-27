import Mathlib

namespace ZMod

def toInt (n : ZMod N): ℤ := n.val

lemma toInt_nonneg [NeZero N] {n : ZMod N}: 0 ≤ n.toInt := by
  rw [toInt]
  apply Int.ofNat_nonneg

lemma toInt_lt [NeZero N] {n : ZMod N}: n.toInt < N := by
  rw [toInt]
  rw [Nat.cast_lt]
  apply ZMod.val_lt

lemma toInt_neg [NeZero N] {n : ZMod N}: (-n).toInt = -(n.toInt) % N := by
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

lemma toInt_add [NeZero N] {n m : ZMod N}: (n + m).toInt = (m.toInt + n.toInt) % N := by
  simp [toInt, val_add, add_comm]

lemma toInt_sub [NeZero N] {n m : ZMod N}: (n - m).toInt = (n.toInt - m.toInt) % N := by
  simp [sub_eq_add_neg, toInt_add, toInt_neg, add_comm]

end ZMod

namespace Int

lemma ofNat_pow {a b : ℕ} : (a^b : ℤ) = (OfNat.ofNat a)^b := by simp [OfNat.ofNat]

theorem negSucc_le_negSucc (m n : Nat) : negSucc m ≤ negSucc n ↔ n ≤ m := by
  rw [le_def]
  apply Iff.intro
  . conv => lhs; arg 1; whnf
    split
    . rename_i h; intro; rw [Nat.succ_sub_succ_eq_sub] at h; exact Nat.le_of_sub_eq_zero h
    . intro; contradiction
  . intro hp;
    conv => arg 1; whnf
    split
    . apply NonNeg.mk
    . rename_i hpc
      linarith [Nat.lt_of_sub_eq_succ hpc]

theorem emod_negSucc (m : Nat) (n : Int) :
  negSucc m % n = subNatNat (natAbs n) (Nat.succ (m % natAbs n)) := rfl

theorem mod_one_below {a : ℤ} {mod : ℤ} (hp : mod > 0) : a < 0 → a ≥ -mod → a % mod = a + mod := by
  intro hlt hge
  have := eq_negSucc_of_lt_zero hlt
  rcases this with ⟨b, ⟨_⟩⟩
  have := eq_succ_of_zero_lt hp
  rcases this with ⟨m, ⟨_⟩⟩
  conv at hge => rhs; whnf
  rw [emod_negSucc]
  simp [natAbs]
  rw [Nat.mod_eq_of_lt]
  rfl
  simp [negSucc_le_negSucc] at hge
  exact Nat.lt_succ_of_le hge

theorem mod_pos_below {a : ℤ} {mod : ℤ} : 0 ≤ a → a < mod → a % mod = a := by
  intros
  simp [emod_eq_of_lt, *]

end Int
