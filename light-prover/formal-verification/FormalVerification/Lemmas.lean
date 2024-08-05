import Mathlib
import «ProvenZk»
import FormalVerification.Circuit

open LightProver (F Order)

axiom bn254_Fr_prime : Nat.Prime Order
instance : Fact (Nat.Prime Order) := Fact.mk bn254_Fr_prime

instance : Membership α (MerkleTree α H d) where
  mem x t := ∃i, t.itemAtFin i = x

namespace ZMod

lemma castInt_lt [NeZero N] {n : ZMod N}: (n:ℤ) < N := by
  rw [cast_eq_val, Nat.cast_lt]
  apply ZMod.val_lt

lemma castInt_nonneg [NeZero N] {n : ZMod N}: (0:ℤ) ≤ n := by
  rw [cast_eq_val]
  apply Int.ofNat_nonneg

lemma castInt_neg [NeZero N] {n : ZMod N}: (((-n): ZMod N) : ℤ) = -(n:ℤ) % N := by
  rw [cast_eq_val, neg_val]
  split
  . simp [*]
  . rw [Nat.cast_sub]
    . rw [←Int.add_emod_self_left, Int.emod_eq_of_lt]
      . simp; rfl
      . linarith [castInt_lt (N:=N)]
      . simp_arith
        rw [ZMod.cast_eq_val, ←Int.ofNat_zero, Int.ofNat_lt]
        apply Nat.zero_lt_of_ne_zero
        simp [*]
    . exact Nat.le_of_lt (ZMod.val_lt _)


lemma castInt_add [NeZero N] {n m : ZMod N}: (((n + m): ZMod N) : ℤ) = ((n:ℤ) + (m:ℤ)) % N := by
  rw [ZMod.cast_eq_val, val_add]
  simp

lemma castInt_sub [NeZero N] {n m : ZMod N}: (((n - m): ZMod N) : ℤ) = ((n:ℤ) - (m:ℤ)) % N := by
  rw [sub_eq_add_neg, castInt_add, castInt_neg]
  simp
  rfl

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

theorem emod_eq_add_self_of_neg_and_lt_neg_self {a : ℤ} {mod : ℤ}: a < 0 → a ≥ -mod → a % mod = a + mod := by
  intro hlt hge
  rw [←add_emod_self]
  apply emod_eq_of_lt
  . linarith
  . linarith

end Int

lemma Membership.get_elem_helper {i n : ℕ} {r : Std.Range} (h₁ : i ∈ r) (h₂ : r.stop = n) :
    i < n := h₂ ▸ h₁.2

macro_rules
| `(tactic| get_elem_tactic_trivial) => `(tactic| (exact Membership.get_elem_helper (by assumption) (by rfl)))

def Std.Range.toList (r : Std.Range): List Nat := go r.start (r.stop - r.start)  where
  go start
  | 0 => []
  | i + 1 => start :: go (start + 1) i

theorem Std.Range.mem_toList_of_mem {r : Std.Range} (hp : i ∈ r) : i ∈ r.toList := by
  rcases hp with ⟨h₁, h₂⟩
  rcases r with ⟨start, stop, _⟩
  simp at h₁ h₂
  have h₃ : ∃d, stop = start + d := by
    exists stop - start
    apply Eq.symm
    apply Nat.add_sub_cancel'
    apply Nat.le_trans h₁ (Nat.le_of_lt h₂)
  rcases h₃ with ⟨d, ⟨_⟩⟩
  induction d generalizing start i with
  | zero => linarith
  | succ d ih =>
    simp [toList, toList.go]
    cases h₁ with
    | refl => tauto
    | @step m h₁ =>
      simp at h₁
      apply Or.inr
      simp [toList] at ih
      apply ih <;> linarith

@[simp]
lemma MerkleTree.GetElem.def {tree : MerkleTree α H d} {i : ℕ} {ih : i < 2^d}:
  tree[i] = tree.itemAtFin ⟨i, ih⟩ := by rfl

theorem Vector.exists_ofElems {p : Fin n → α → Prop} : (∀ (i : Fin n), ∃j, p i j) ↔ ∃(v : Vector α n), ∀i (_: i<n), p ⟨i, by assumption⟩ v[i] := by
  apply Iff.intro
  . intro h
    induction n with
    | zero =>
      exists Vector.nil
      intro i h
      linarith [h]
    | succ n ih =>
      rw [Vector.exists_succ_iff_exists_snoc]
      have hp_init := ih fun (i : Fin n) => h (Fin.castLE (by linarith) i)
      rcases hp_init with ⟨vinit, hpinit⟩
      exists vinit
      have hp_last := h (Fin.last n)
      rcases hp_last with ⟨vlast, hplast⟩
      exists vlast
      intro i ihp
      cases Nat.lt_succ_iff_lt_or_eq.mp ihp with
      | inl ihp =>
        simp [ihp]
        apply hpinit
      | inr ihp =>
        simp [ihp]
        apply hplast
  . rintro ⟨v, h⟩ i
    exact ⟨v[i], h i i.2⟩
